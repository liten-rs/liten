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
