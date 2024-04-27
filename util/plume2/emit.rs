pub trait EmitFn<T> {
    fn emit(&mut self, value: T);
    fn finish(self: Box<Self>);
}
