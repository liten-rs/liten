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
    let ipv4 = Ipv4Addr::from(unsafe { *ipv4_ptr }.sin_addr.s_addr);
    let port = u16::from_be(unsafe { *ipv4_ptr }.sin_port);

    Ok(SocketAddr::from(SocketAddrV4::new(ipv4, port)))
  } else if sockaddr.ss_family == libc::AF_INET6 as libc::sa_family_t {
    let ipv6_ptr = storage.cast::<libc::sockaddr_in6>();
    let in6 = unsafe { *ipv6_ptr };
    let ipv6 = Ipv6Addr::from(in6.sin6_addr.s6_addr);
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
