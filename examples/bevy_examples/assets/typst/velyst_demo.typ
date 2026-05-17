// Simple 32-bit LCG for deterministic dot placement.
#let lcg(s) = calc.rem(s * 1103515245 + 12345, 2147483648)
#let to-f(s) = lcg(s) / 2147483648.0

// Box-Muller normal variate; returns (next_state, z).
#let randn(s) = {
  let s1 = lcg(s)
  let s2 = lcg(s1)
  let u1 = calc.max(to-f(s1), 1e-7)
  let u2 = to-f(s2)
  (s2, calc.sqrt(-2.0 * calc.ln(u1)) * calc.cos(2 * calc.pi * u2))
}

#import "monokai_pro.typ": *

#let sigma = 85
#let amplitude = 190
#let baseline-off = 100
#let x-range = 295
#let n-dots = 80
#let dot-r = 4pt

#let gaussian(x) = calc.exp(-0.5 * (x / sigma) * (x / sigma))

#let (_, dot-data) = range(n-dots).fold(
  (57005, ()),
  ((s, acc), _) => {
    let (s1, xn) = randn(s)
    let (s2, noise) = randn(s1)
    let sx = calc.clamp(xn * sigma, -x-range, x-range)
    let g = gaussian(sx)
    let sy = baseline-off - amplitude * g + noise * 15
    (s2, acc + ((sx, sy),))
  },
)

#let plot() = {
  let w = 640pt
  let h = 440pt
  let cx = w / 2
  let cy = h / 2
  let baseline = cy + baseline-off * 1pt
  let y-top = cy + (baseline-off - amplitude - 20) * 1pt
  let axis-stroke = base4 + 1pt

  box(width: w, height: h)[
    // Axes.
    #place(left + top, dx: cx - x-range * 1pt - 8pt, dy: baseline, line(
      start: (0pt, 0pt),
      end: ((2 * x-range + 16) * 1pt, 0pt),
      stroke: axis-stroke,
    ))
    #place(left + top, dx: cx, dy: y-top, line(
      start: (0pt, 0pt),
      end: (0pt, baseline - y-top + 8pt),
      stroke: axis-stroke,
    ))

    // Scatter dots; each circle becomes a separate Kanva path inside the group.
    #box(width: w, height: h)[
      #for pos in dot-data {
        let sx = pos.at(0) * 1pt
        let sy = pos.at(1) * 1pt
        place(left + top, dx: cx + sx - dot-r, dy: cy + sy - dot-r, circle(
          radius: dot-r,
          fill: orange,
        ))
      }
    ] <dots>

    // Full Gaussian curve; shape is overridden each frame for path tracing.
    #let pts = range(301).map(i => {
      let t = i / 300
      let x = -x-range + 2 * x-range * t
      let g = gaussian(x)
      let y = baseline-off - amplitude * g
      (cx + x * 1pt, cy + y * 1pt)
    })
    #let curve-cmds = (
      (curve.move(pts.at(0)),)
        + range(1, pts.len()).map(i => curve.line(pts.at(i)))
    )
    #box(width: w, height: h)[
      #place(left + top, curve(
        stroke: blue + 2.5pt,
        ..curve-cmds,
      ))
    ] <curve>
  ]
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
