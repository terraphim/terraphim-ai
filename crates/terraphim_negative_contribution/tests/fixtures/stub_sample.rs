fn main() {
    todo!()
}

fn bar() {
    unimplemented!()
}

fn baz() {
    todo!() // terraphim: allow(stub)
}

fn qux() {
    panic!("not implemented")
}

fn real_work() -> i32 {
    42
}
