pub struct FromNextFn<A, F: FnMut() -> Option<A>> {
    f: F,
}

impl<A, F: FnMut() -> Option<A>> FromNextFn<A, F> {
    pub fn new(f: F) -> Self {
        FromNextFn {
            f: f,
        }
    }
}

impl<A, F: FnMut() -> Option<A>> Iterator for FromNextFn<A, F> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        (self.f)()
    }
}
