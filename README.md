# lilscript

Designed for handling different formats of [lilellia's ASMR RP scripts](https://scriptbin.works/u/_lilell_).

## Usage

```bash
cargo run -- --infile=/path/to/script.tex --output=/path/to/export.md
```

`--infile` (or `-i`) and `--outfile` (or `-o`) can be either .tex or .md, with the caveat that only tex ⟶ Script ⟶ md is currently supported.

## Features

- [x] Parsing .tex file to an internal Script format
- [ ] Parsing .md file to internal Script format
- [ ] Exporting internal Script format to .tex file
- [ ] Add .tex/.md conversion to PDF
- [x] Exporting internal Script format to .md file
- [x] Determining word count for script (spoken words, total words)...
- [x] ...and the corresponding speech density
