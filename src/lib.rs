pub trait FromUnsafe<T> {
    unsafe fn from(T) -> Self;
}

pub mod types;
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
