//! MathML elements.

macro_rules! generate_math_tags {
  ($($tt:tt)*) => {};
}

generate_math_tags![
  /// MathML element.
  math,
  /// MathML element.
  mi,
  /// MathML element.
  mn,
  /// MathML element.
  mo,
  /// MathML element.
  ms,
  /// MathML element.
  mspace,
  /// MathML element.
  mtext,
  /// MathML element.
  menclose,
  /// MathML element.
  merror,
  /// MathML element.
  mfenced,
  /// MathML element.
  mfrac,
  /// MathML element.
  mpadded,
  /// MathML element.
  mphantom,
  /// MathML element.
  mroot,
  /// MathML element.
  mrow,
  /// MathML element.
  msqrt,
  /// MathML element.
  mstyle,
  /// MathML element.
  mmultiscripts,
  /// MathML element.
  mover,
  /// MathML element.
  mprescripts,
  /// MathML element.
  msub,
  /// MathML element.
  msubsup,
  /// MathML element.
  msup,
  /// MathML element.
  munder,
  /// MathML element.
  munderover,
  /// MathML element.
  mtable,
  /// MathML element.
  mtd,
  /// MathML element.
  mtr,
  /// MathML element.
  maction,
  /// MathML element.
  annotation,
  /// MathML element.
  annotation
    - xml,
  /// MathML element.
  semantics,
];
