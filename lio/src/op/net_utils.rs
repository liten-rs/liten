use std::{
  cell::UnsafeCell,
  io, mem,
  net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
  ptr,
};

pub fn libc_socketaddr_into_std(
  storage: *const libc::sockaddr_storage,
) -> io::Result<SocketAddr> {
  let sockaddr = unsafe { *storage };

  if sockaddr.ss_family == libc::AF_INET as libc::sa_family_t {
    let ipv4_ptr = storage.cast::<libc::sockaddr_in>();
    let ipv4 = Ipv4Addr::from(unsafe { *ipv4_ptr }.sin_addr.s_addr.to_be());
    let port = u16::from_be(unsafe { *ipv4_ptr }.sin_port);

    Ok(SocketAddr::from(SocketAddrV4::new(ipv4, port)))
  } else if sockaddr.ss_family == libc::AF_INET6 as libc::sa_family_t {
    let ipv6_ptr = storage.cast::<libc::sockaddr_in6>();
    let in6 = unsafe { *ipv6_ptr };
    let ipv6 =
      Ipv6Addr::from(u128::from_le_bytes(in6.sin6_addr.s6_addr).to_be());
    let port = u16::from_be(in6.sin6_port);

    Ok(SocketAddr::from(SocketAddrV6::new(
      ipv6,
      port,
      in6.sin6_flowinfo,
      in6.sin6_scope_id,
    )))
  } else {
    Err(io::Error::from_raw_os_error(libc::EAFNOSUPPORT))
  }
}

pub fn std_socketaddr_into_libc(addr: SocketAddr) -> libc::sockaddr_storage {
  let storage: UnsafeCell<libc::sockaddr_storage> =
    UnsafeCell::new(unsafe { mem::zeroed() });
  match addr {
    SocketAddr::V4(v4) => unsafe {
      // We copy the bytes from the source pointer (&v4)
      // to the destination pointer (&mut storage)
      ptr::copy_nonoverlapping(
        &into_addr(v4) as *const _ as *const u8,
        storage.get() as *mut u8,
        // We calculate the size of the IPv4 address structure
        mem::size_of::<libc::sockaddr_in>(),
      );
    },
    SocketAddr::V6(v6) => unsafe {
      // We copy the bytes from the source pointer (&v6)
      // to the destination pointer (&mut storage)
      ptr::copy_nonoverlapping(
        &into_addr6(v6) as *const _ as *const u8,
        storage.get() as *mut u8,
        // We calculate the size of the IPv6 address structure
        mem::size_of::<libc::sockaddr_in6>(),
      );
    },
  };

  storage.into_inner()
}

/// Converts a raw libc::sockaddr pointer and length into a safe std::net::SocketAddr.
///
/// # Safety
///
/// This function is highly unsafe. The caller must ensure:
/// 1. `raw_addr_ptr` is a valid, non-null pointer to a properly initialized `sockaddr` struct.
/// 2. The memory pointed to is valid for reading for the duration of the function call.
/// 3. The length `addr_len` correctly specifies the size of the underlying structure (e.g., sizeof(sockaddr_in)).
#[cfg(feature = "unstable_ffi")]
pub fn sockaddr_to_socketaddr(
  raw_addr_ptr: *const libc::sockaddr,
  addr_len: libc::socklen_t,
) -> Option<SocketAddr> {
  if raw_addr_ptr.is_null() {
    return None;
  }

  // Read the address family field to determine the actual type
  let family = unsafe { *raw_addr_ptr }.sa_family as i32;

  match family {
    libc::AF_INET => {
      // Check length consistency (optional but good practice)
      if addr_len < mem::size_of::<libc::sockaddr_in>() as libc::socklen_t {
        return None;
      }

      // Cast the general pointer to a specific IPv4 pointer
      let raw_v4 = raw_addr_ptr as *const libc::sockaddr_in;
      let sin_addr = unsafe { *raw_v4 }.sin_addr;
      let sin_port = unsafe { *raw_v4 }.sin_port;

      // Convert network byte order to host byte order for port
      let port = u16::from_be(sin_port);

      // `s_addr` is a u32 in network byte order; Ipv4Addr::from handles the conversion
      let ipv4_addr = Ipv4Addr::from(u32::from_be(sin_addr.s_addr));

      Some(SocketAddr::V4(SocketAddrV4::new(ipv4_addr, port)))
    }
    libc::AF_INET6 => {
      // Check length consistency (optional but good practice)
      if addr_len < mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t {
        return None;
      }

      // Cast the general pointer to a specific IPv6 pointer
      let raw_v6 = raw_addr_ptr as *const libc::sockaddr_in6;
      let sin6_addr = unsafe { *raw_v6 }.sin6_addr;
      let sin6_port = unsafe { *raw_v6 }.sin6_port;
      let sin6_flowinfo = unsafe { *raw_v6 }.sin6_flowinfo;
      let sin6_scope_id = unsafe { *raw_v6 }.sin6_scope_id;

      // Convert network byte order to host byte order for port
      let port = u16::from_be(sin6_port);

      // `s6_addr` is a [u8; 16] array; Ipv6Addr::from handles this directly
      let ipv6_addr = Ipv6Addr::from(sin6_addr.s6_addr);

      Some(SocketAddr::V6(SocketAddrV6::new(
        ipv6_addr,
        port,
        u32::from_be(sin6_flowinfo), // Flow info and scope ID might need byte swap depending on platform/libc version
        u32::from_be(sin6_scope_id),
      )))
    }
    _ => {
      // Address family is neither IPv4 nor IPv6 (e.g., AF_UNIX, AF_BLUETOOTH)
      None
    }
  }
}

fn into_addr(addr: SocketAddrV4) -> libc::sockaddr_in {
  let mut _addr: libc::sockaddr_in = unsafe { mem::zeroed() };

  #[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
  ))]
  {
    _addr.sin_len = mem::size_of::<libc::sockaddr_in>() as u8;
  }
  _addr.sin_family = libc::AF_INET as libc::sa_family_t;
  _addr.sin_port = addr.port().to_be();
  _addr.sin_addr = libc::in_addr { s_addr: u32::from(*addr.ip()).to_be() };

  _addr
}

fn into_addr6(addr: SocketAddrV6) -> libc::sockaddr_in6 {
  let mut _addr: libc::sockaddr_in6 = unsafe { mem::zeroed() };

  #[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
  ))]
  {
    _addr.sin6_len = mem::size_of::<libc::sockaddr_in6>() as u8;
  }
  _addr.sin6_family = libc::AF_INET6 as libc::sa_family_t;
  _addr.sin6_port = addr.port().to_be();
  _addr.sin6_addr = libc::in6_addr { s6_addr: addr.ip().octets() };

  _addr
}
