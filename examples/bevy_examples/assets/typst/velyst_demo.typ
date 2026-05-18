#import "@preview/cetz:0.5.2": canvas, draw
#import "monokai_pro.typ": *

#let plot(circle_x, circle_y) = {
  let half_w = 1000
  let half_h = 1000
  let grid_step = 40

  canvas(length: 1pt, padding: 0pt, {
    import draw: *
    content((0, 0), [#box() <grid-start>])
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
    content((0, 0), [#box() <grid-end>])

    content((0, 0), [#box() <circle-start>])
    circle(
      (circle_x * grid_step, circle_y * grid_step),
      radius: 20pt,
      fill: red,
      stroke: red.lighten(50%) + 2pt,
    )
    content((0, 0), [#box() <circle-end>])
  })
}

#let equation() = box(inset: (y: 1.2em))[
  #set text(size: 24pt, fill: base8, stroke: 20pt + base8)

  #box()[$f(x) = frac(1, sigma sqrt(2 pi)) e^(-frac((x - mu)^2, 2 sigma^2))$] <coord>
]
