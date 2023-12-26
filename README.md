# cow
Discord bot for the UC-Mooced Discord. Written by William and Andrew in Rust.

### Building Map Download Instructions.
There's no pure-rust library for rendering pdfs to images. 

You need to get pdfium from Chromium. 
Either build from source or download the prebuilt .so [file](https://github.com/bblanchon/pdfium-binaries?tab=readme-ov-file)

And make sure LD_LIBRARY_PATH is set to where the .so file is (or place it in /usr/lib)