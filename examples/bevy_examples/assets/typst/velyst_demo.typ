#import "@preview/cetz:0.5.2": canvas, draw
#import "monokai_pro.typ": *

// Simple 32-bit LCG for deterministic dot placement.
#let lcg(s) = calc.rem(s * 1103515245 + 12345, 2147483648)
#let to_f(s) = lcg(s) / 2147483648.0

// Box-Muller transform; returns (next_state, z) where z ~ N(0, 1).
#let randn(s) = {
  let s1 = lcg(s)
  let s2 = lcg(s1)
  let u1 = calc.max(to_f(s1), 1e-7)
  let u2 = to_f(s2)
  (s2, calc.sqrt(-2.0 * calc.ln(u1)) * calc.cos(2 * calc.pi * u2))
}

#let std = 85
#let amplitude = 190
#let baseline_off = 100
#let x_range = 295
#let n_dots = 80
#let dot_r = 4

#let gaussian(x) = calc.exp(-0.5 * (x / std) * (x / std))

#let (_, dot_data) = range(n_dots).fold(
  (57005, ()),
  ((s, acc), _) => {
    let (s1, xn) = randn(s)
    let (s2, noise) = randn(s1)
    let sx = calc.clamp(xn * std, -x_range, x_range)
    let g = gaussian(sx)
    let sy = baseline_off - amplitude * g + noise * 15
    (s2, acc + ((sx, sy),))
  },
)

#let plot() = {
  let half_w = 1000
  let half_h = 1000
  let grid_step = 40

  set block(clip: false)
  set box(clip: false)

  canvas(length: 1pt, padding: 0pt, {
    import draw: *
    content((0, 0), [#box()<grid-start>])
    grid(
      (-half_w, -half_h),
      (half_w, half_h),
      step: grid_step,
      stroke: base2 + 0.5pt,
    )
    grid(
      (-half_w, -half_h),
      (half_w, half_h),
      step: grid_step * 5,
      stroke: blue + 0.5pt,
    )
    content((0, 0), [#box()<grid-end>])
    circle((0, 0), radius: 50pt, fill: red)
  })

  // c
  // move(dx: -half_w * 1pt / 2, dy: -half_h * 1pt / 2)[#c]
}

#let equation() = box(
  fill: base0,
  stroke: base1 + 1pt,
  inset: 20pt,
  radius: 8pt,
)[
  #set text(size: 24pt, fill: base8, stroke: 20pt + base8)

  $f(x) = frac(1, sigma sqrt(2 pi)) e^(-frac((x - mu)^2, 2 sigma^2))$

  #v(6pt)
  #set text(size: 11pt)
  #grid(
    columns: (20pt, 1fr),
    column-gutter: 4pt,
    row-gutter: 3pt,
    $mu$, [mean],
    $sigma$, [standard deviation],
    $x$, [observation],
  )
]
