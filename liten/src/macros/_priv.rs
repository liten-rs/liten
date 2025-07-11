macro_rules! cfg_time {
   ($($item:item)*) => {
       $(
            #[cfg(feature = "time")]
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

macro_rules! cfg_actor {
   ($($item:item)*) => {
       $(
            #[cfg(feature = "actor")]
            #[cfg_attr(docsrs, doc(cfg(feature = "actor")))]
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
