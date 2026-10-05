# UI1.1a font and raster assertions

These are immutable assertion inputs and independently prepared expectations,
not evidence that an editor or compositor works. The host API fixes an 8×16
MSB-left glyph cell and opaque BGRA colors. The font covers printable ASCII
32–126; slot 127 is blank and unsupported. No runtime font dependency is added.

`font8x8_basic.pinned.h` preserves the complete upstream source and its public
domain notice from [Daniel Hepper's font8x8, pinned commit
8e279d2](https://github.com/dhepper/font8x8/blob/8e279d2d864e79128e96188a6b9526cfa3fbfef9/font8x8_basic.h).
The [upstream encoding description](https://github.com/dhepper/font8x8/blob/8e279d2d864e79128e96188a6b9526cfa3fbfef9/README)
defines bit 0 as the leftmost pixel. For `ascii8x16_v0.bin`, take codepoints
32–127 in order, reverse each row's eight bits, and repeat each row twice.
The result is exactly 96×16=1536 bytes. Digests and layout live in
`ascii8x16_v0.provenance.json`.

`ascii8x16_v0.golden.json` pins the 640×480 app crop for `note=old\n` and
`note=new\n`: white opaque background, black opaque glyphs, no selection,
cursor at byte offset 9 on the next line, and the last-painted 2×16 black
cursor at x0/y16. Samples use crop coordinates; composed coordinates add
48 to y. These full-crop digests were prepared independently before handlers.
Tests must compare actual consumed pixels against these fixed expectations,
not derive an expected hash by calling the production renderer. Chrome,
indicator, selection, scrolling and adversarial clipping require their own
observed assertions. Fixture files cannot certify rendering or containment.
