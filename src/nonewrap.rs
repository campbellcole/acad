use std::fmt::Display;

pub trait Nonewrap {
    type Output;

    fn nonewrap(self) -> Option<Self::Output>;
}

impl<T, E: Display> Nonewrap for Result<T, E> {
    type Output = T;

    fn nonewrap(self) -> Option<Self::Output> {
        match self {
            Ok(v) => Some(v),
            Err(err) => {
                error!("{}", err);
                None
            }
        }
    }
}
