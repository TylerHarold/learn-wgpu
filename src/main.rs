use learn_wgpu::run; 

/*
*   Since our run is async, main() will need some way to await the future. We coukld use a crate
*   like tokio or async-std, but this example we'll go with the much more lightweight pollster. 
*/
fn main() {
    // Don't use block_on inside of an async function if we plan to support WASM. Futures have to
    // be run using the browser's executor. If you try to bring your own your code will crash
    // when you encounter a future that doesn't execute immediately.
    pollster::block_on(run());
}
