# Example

```rust
use spectrograph::{
    rgb_spectrograph_color_f32,
    SpectrumFrame,
    SpectrographOptions,
    Normalizer,
    Colormap,
};

fn main() {
    // Assume `stft` was produced earlier by your STFT or CWT engine.
    // It contains:
    // - stft.data  (2D frequency x time buffer)
    // - stft.width (number of frames)
    // - stft.height (number of frequency bins)

    let image = rgb_spectrograph_color_f32(
        &SpectrographFrame {
            data: std::borrow::Cow::Borrowed(stft.data.borrow()),
            width: stft.width,
            height: stft.height,
        },
        SpectrographOptions {
            out_width: 1920,
            out_height: 1080,
            normalizer: Normalizer::LogMagnitude,
            colormap: Colormap::Inferno,
        },
    )
    .unwrap();

    // `image` is a Vec<u8> in RGB format (width * height * 3)
    // You can now save it using your preferred image library.
}
```

----

This project is licensed under either of

- BSD-3-Clause License (see [LICENSE](LICENSE.md))
- Apache License, Version 2.0 (see [LICENSE](LICENSE-APACHE.md))

at your option.