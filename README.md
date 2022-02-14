# hanziscraper

scrape the 汉字 from hanzidb.org into a local file.

to run locally:
1. set up rust and cargo
2. install [opencc](https://github.com/BYVoid/OpenCC)
    - an open source project for converting chinese characters to different forms (traditional, japanese, etc)
3. compile with the command `cargo build`

usage:
`./hanziscraper <output_file_path>`
