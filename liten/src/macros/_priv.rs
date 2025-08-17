macro_rules! cfg_time {
   ($($item:item)*) => {
       $(
            #[cfg(all(feature = "time", not(loom)))]
            #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
            $item
        )*
    }
}

macro_rules! cfg_blocking {
   ($($item:item)*) => {
       $(
            #[cfg(feature = "blocking")]
            #[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
            $item
        )*
    }
}

macro_rules! cfg_rt {
   ($($item:item)*) => {
       $(
            #[cfg(feature = "runtime")]
            #[cfg_attr(docsrs, doc(cfg(feature = "runtime")))]
            $item
        )*
    }
}

macro_rules! cfg_sync {
   ($($item:item)*) => {
       $(
            #[cfg(feature = "sync")]
            #[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
            $item
        )*
    }
}

macro_rules! cfg_fs {
   ($($item:item)*) => {
       $(
            #[cfg(feature = "fs")]
            #[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
            $item
        )*
    }
}

macro_rules! cfg_io {
   ($($item:item)*) => {
       $(
            #[cfg(feature = "io")]
            #[cfg_attr(docsrs, doc(cfg(feature = "io")))]
            $item
        )*
    }
}

macro_rules! cfg_not_coro {
   ($($item:item)*) => {
       $(
            #[cfg(all(not(feature = "runtime"), feature = "coro"))]
            #[cfg_attr(docsrs, doc(cfg(all(not(feature = "runtime"), feature = "coro"))))]
            $item
        )*
    }
}

macro_rules! cfg_coro {
   ($($item:item)*) => {
       $(
            #[cfg(all(feature = "runtime", feature = "coro"))]
            #[cfg_attr(docsrs, doc(cfg(all(feature = "runtime", feature = "coro"))))]
            $item
        )*
    }
}

macro_rules! cfg_compat {
   ($($item:item)*) => {
       $(
            #[cfg(feature = "compat")]
            #[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
            $item
        )*
    }
}
