package main

import (
	"bytes"
	"image"
	"image/color"
	"image/png"
)

// makeIcon returns a 22×22 PNG for the system tray.
// Draws a pixel-art "S" in Catppuccin Sky (#89dceb) on
// a Catppuccin Base (#1e1e2e) background.
func makeIcon() []byte {
	const size = 22
	img := image.NewNRGBA(image.Rect(0, 0, size, size))

	bg := color.NRGBA{R: 0x1e, G: 0x1e, B: 0x2e, A: 0xff}
	fg := color.NRGBA{R: 0x89, G: 0xdc, B: 0xeb, A: 0xff}

	for y := 0; y < size; y++ {
		for x := 0; x < size; x++ {
			img.SetNRGBA(x, y, bg)
		}
	}

	// 8×9 pixel-art "S", origin at (7, 6)
	glyph := []string{
		" XXXXXX",
		"X      ",
		"X      ",
		" XXXXX ",
		"      X",
		"      X",
		"XXXXXX ",
	}
	ox, oy := 7, 7
	for gy, row := range glyph {
		for gx, ch := range row {
			if ch == 'X' {
				img.SetNRGBA(ox+gx, oy+gy, fg)
			}
		}
	}

	var buf bytes.Buffer
	_ = png.Encode(&buf, img)
	return buf.Bytes()
}
