# win-streamshot

Adaptation of the win-screenshot crate, but for taking continuous screenshots.

It uses the windows crate under the hood to take screenshots.

## Usage

```rust
fn main() {
    let window_finder = WindowFinder::new();
    let firefox = window_finder.find("Firefox").unwrap();
    let screenshot = firefox.get_rgb_screenshot().unwrap();
    println!("{}x{}", screenshot.width(), screenshot.height());
}
```
