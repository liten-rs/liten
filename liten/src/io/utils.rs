pub mod net {
  use std::{
    io::{self, ErrorKind},
    mem,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
  };

  use libc as c;

  // Copied from std::sys::net 154 (rustc 1.88)

  fn ip_v4_addr_to_c(addr: &Ipv4Addr) -> c::in_addr {
    // `s_addr` is stored as BE on all machines and the array is in BE order.
    // So the native endian conversion method is used so that it's never swapped.
    c::in_addr { s_addr: u32::from_ne_bytes(addr.octets()) }
  }

  fn ip_v6_addr_to_c(addr: &Ipv6Addr) -> c::in6_addr {
    c::in6_addr { s6_addr: addr.octets() }
  }

  fn ip_v4_addr_from_c(addr: c::in_addr) -> Ipv4Addr {
    Ipv4Addr::from(addr.s_addr.to_ne_bytes())
  }

  fn ip_v6_addr_from_c(addr: c::in6_addr) -> Ipv6Addr {
    Ipv6Addr::from(addr.s6_addr)
  }

  fn socket_addr_v4_to_c(addr: &SocketAddrV4) -> c::sockaddr_in {
    c::sockaddr_in {
      sin_family: c::AF_INET as c::sa_family_t,
      sin_port: addr.port().to_be(),
      sin_addr: ip_v4_addr_to_c(addr.ip()),
      ..unsafe { mem::zeroed() }
    }
  }

  fn socket_addr_v6_to_c(addr: &SocketAddrV6) -> c::sockaddr_in6 {
    c::sockaddr_in6 {
      sin6_family: c::AF_INET6 as c::sa_family_t,
      sin6_port: addr.port().to_be(),
      sin6_addr: ip_v6_addr_to_c(addr.ip()),
      sin6_flowinfo: addr.flowinfo(),
      sin6_scope_id: addr.scope_id(),
      ..unsafe { mem::zeroed() }
    }
  }

  /// A type with the same memory layout as `c::sockaddr`. Used in converting Rust level
  /// SocketAddr* types into their system representation. The benefit of this specific
  /// type over using `c::sockaddr_storage` is that this type is exactly as large as it
  /// needs to be and not a lot larger. And it can be initialized more cleanly from Rust.
  #[repr(C)]
  pub union SocketAddrCRepr {
    v4: c::sockaddr_in,
    v6: c::sockaddr_in6,
  }

  impl SocketAddrCRepr {
    pub fn as_ptr(&self) -> *const c::sockaddr {
      self as *const _ as *const c::sockaddr
    }
  }

  pub fn socket_addr_to_c(
    addr: &SocketAddr,
  ) -> (SocketAddrCRepr, c::socklen_t) {
    match addr {
      SocketAddr::V4(a) => {
        let sockaddr = SocketAddrCRepr { v4: socket_addr_v4_to_c(a) };
        (sockaddr, size_of::<c::sockaddr_in>() as c::socklen_t)
      }
      SocketAddr::V6(a) => {
        let sockaddr = SocketAddrCRepr { v6: socket_addr_v6_to_c(a) };
        (sockaddr, size_of::<c::sockaddr_in6>() as c::socklen_t)
      }
    }
  }

  pub unsafe fn socket_addr_from_c(
    storage: *const c::sockaddr_storage,
    len: usize,
  ) -> io::Result<SocketAddr> {
    match (*storage).ss_family as libc::c_int {
      c::AF_INET => {
        assert!(len >= size_of::<c::sockaddr_in>());
        Ok(SocketAddr::V4(socket_addr_v4_from_c(unsafe {
          *(storage as *const _ as *const c::sockaddr_in)
        })))
      }
      c::AF_INET6 => {
        assert!(len >= size_of::<c::sockaddr_in6>());
        Ok(SocketAddr::V6(socket_addr_v6_from_c(unsafe {
          *(storage as *const _ as *const c::sockaddr_in6)
        })))
      }
      _ => Err(io::Error::new(ErrorKind::InvalidInput, "invalid argument")),
    }
  }
  fn socket_addr_v4_from_c(addr: c::sockaddr_in) -> SocketAddrV4 {
    SocketAddrV4::new(
      ip_v4_addr_from_c(addr.sin_addr),
      u16::from_be(addr.sin_port),
    )
  }

  fn socket_addr_v6_from_c(addr: c::sockaddr_in6) -> SocketAddrV6 {
    SocketAddrV6::new(
      ip_v6_addr_from_c(addr.sin6_addr),
      u16::from_be(addr.sin6_port),
      addr.sin6_flowinfo,
      addr.sin6_scope_id,
    )
  }
}
