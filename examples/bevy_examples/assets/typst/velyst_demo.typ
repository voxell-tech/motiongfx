#import "@preview/cetz:0.5.2": canvas, draw
#import "monokai_pro.typ": *

#let plot(circle_x, circle_y) = {
  let half_w = 1000
  let half_h = 1000
  let grid_step = 40

  let circle_size = 20
  let matrix_offset = circle_size + 10

  let circle_coord = (circle_x * grid_step, circle_y * grid_step)
  let matrix_coord = (
    circle_coord.at(0) + matrix_offset,
    circle_coord.at(1) + matrix_offset,
  )

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
      circle_coord,
      radius: circle_size,
      stroke: purple.lighten(50%) + 2pt,
      fill: purple,
    )
    content((0, 0), [#box() <circle-end>])

    if not (
      circle_coord.at(0) < grid_step - circle_size
        and circle_coord.at(1) < grid_step - circle_size
    ) {
      content((0, 0), [#box() <arrow-start>])
      line(
        (0, 0),
        circle_coord,
        mark: (end: ">"),
        stroke: red + 10pt,
      )
      content((0, 0), [#box() <arrow-end>])
    }

    content((0, 0), [#box() <coord-start>])
    content(
      matrix_coord,
      box(inset: (y: 1.2em))[
        #set text(size: 24pt, fill: base8, stroke: 20pt + base8)
        $mat(delim: "[", #calc.round(circle_x) ; #calc.round(circle_y))$
      ],
    )
    content((0, 0), [#box() <coord-end>])
  })
}
