
const PREC = {
  COMMA: -1,
  ASSIGN: 0,
  COMMENT: 1,
  VAR: 2,
};

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}

function maybeCommaSep(rule) {
  return repeat(seq(rule, optional(',')))
}

function constructedType(name, rule) {
  return seq(name, "(", rule, ")")
}

const grammarObject = {
  name: 'terraform',

  extras: $ => [
    $.comment,
    /[\s\uFEFF\u2060\u200B\u00A0]/,
  ],

  rules: {
    configuration: $ => repeat(choice(
      $.resource,
      $.data,
      $.provider,
      $.variable,
      $.output,
      $.locals,
      $.module,
    )),

    variable: $ => seq(
      'variable',
      alias($.string_literal, $.variable_name),
      $.variable_block,
    ),

    variable_block: $ => seq('{',
      optional($.type),
      optional($._description),
      optional($.default),
    '}'),

    _description: $ => seq("description", "=", alias($.string_literal, $.description)),

    default: $ => seq("default", "=", $._expression),

    type: $ => seq("type", "=", $._types),

    _types: $ => choice(
      alias("bool", $.bool_ty),
      alias("string", $.string_ty),
      alias("number", $.number_ty),
      $.list_ty,
      $.set_ty,
      $.map_ty,
      $.object_ty,
      $.tuple_ty,
    ),

    list_ty: $ => constructedType("list", $._types),
    set_ty: $ => constructedType("set", $._types),
    map_ty: $ => constructedType("map", $._types),
    object_ty: $ => seq( 'object',
      '(',
      '{',
        maybeCommaSep($.object_field),
      '}',
      ')'
    ),
    object_field: $ => seq(alias($.identifier, $.field_name), "=", $._types),
    tuple_ty: $ => seq('tuple', '(', '[', commaSep($._types), ']', ')'),


    provider: $ => seq('provider', alias($.string_literal, $.provider_name), $.block),

    output: $ => seq('output', alias($.string_literal, $.output_name), $.block),

    module: $ => seq('module', alias($.string_literal, $.module_name), $._module_definition),

    _module_definition: $ => seq('{', $._source,  repeat($.attribute), '}'),

    _source: $ => seq('source', '=', alias($.string_literal, $.source)),

    resource: $ => seq(
      'resource',
      alias($.string_literal, $.resource_type),
      alias($.string_literal, $.resource_name),
      $.block,
    ),

    data: $ => seq(
      'data',
      alias($.string_literal, $.data_type),
      alias($.string_literal, $.data_name),
      $.block,
    ),

    locals: $ => seq(
      'locals',
      $.block,
    ),

    attribute: $ => choice(
      prec.right(PREC.ASSIGN, seq($.identifier, $._initializer)),
      $.named_map,
    ),

    _expression: $ => choice(
      $.string_literal,
      $.boolean,
      $.number,
      $.interpolation_string,
      alias($.block, $.map),
      $.list,
      $._operation,
      prec(10, $.function),
      $.reference,
    ),

    _operation: $ => choice(
      $.ternary,
    ),

    ternary: $ => seq($._booleanExpression, "?", $._expression, ":", $._expression),

    _booleanExpression: $ => choice(
      $.comparison,
    ),

    comparison: $ => prec(10, seq($._expression, $._comparisonOperator , $._expression)),

    _comparisonOperator: $ => choice(
      alias("==", $.eq),
      alias(">", $.gt),
      alias("<", $.lt),
    ),

    function: $ => seq(choice(
      "merge",
      "length",
    ),
    "(", repeat(seq($.fn_param, optional(','))), ")"),

    fn_param: $ => $._expression,


    _initializer: $ => seq(
      '=',
      $._expression,
    ),

    named_map: $ => seq(
      $.identifier,
      alias($.block, $.map),
    ),

    list: $ => seq(
      '[',
      commaSep($._expression),
      optional(','),
      ']',
    ),

    identifier: ($) => {
      const alpha = /[a-zA-Z]+/;
      const alphaNumeric = /[a-zA-Z0-9-_]+/;

      return token(seq(alpha, repeat(alphaNumeric)));
    },

    reference: $ => {
      const alpha = /[a-zA-Z]/;
      const alphaNumeric = /[a-zA-Z0-9-_\.]+/;

      return token(seq(alpha, repeat(alphaNumeric)));
    },

    comment: $ => token(prec(PREC.COMMENT, choice(
      seq('#', /.*/),
      seq('//', /.*/),
      seq(
        '/*',
        /[^*]*\*+([^/*][^*]*\*+)*/,
        '/',
      ),
    ))),


    block: $ => seq(
      '{',
      repeat($.attribute),
      '}',
    ),

    boolean: $ => choice('true', 'false'),

    number: ($) => {
      const decimalDigits = /\d+/;
      const hexLiteral = seq('0x', /[\da-fA-F]+/);

      const decimalIntegerLiteral = choice(
        '0',
        seq(optional('-'), optional('0'), /[1-9]/, optional(decimalDigits)),
      );

      const signedInteger = seq(
        optional(choice('-', '+')),
        decimalDigits,
      );

      const exponentPart = seq(choice('e', 'E'), signedInteger);

      const decimalLiteral = choice(
        seq(decimalIntegerLiteral, '.', optional(decimalDigits), optional(exponentPart)),
        seq('.', decimalDigits, optional(exponentPart)),
        seq(decimalIntegerLiteral, optional(exponentPart)),
      );
      return token(choice(
        decimalLiteral,
        hexLiteral,
      ));
    },

    interpolation_string: $ => seq(
      '"',
      repeat(choice(
        $._template_chars,
        $.interpolation_substitution,
      )),
      '"',
    ),

    interpolation_substitution: $ => seq(
      '${',
      $._expressions,
      '}',
    ),

    _expressions: $ => choice(
      $._expression,
      $.sequence_expression,
    ),

    _template_chars: $ => token(choice(repeat1(choice(
      /[^\\"$]/,
      /\$[^{"$]/,
      /\\(.|\n)/,
    )))),

    sequence_expression: $ => prec(PREC.COMMA, seq(
      $._expression,
      ',',
      choice(
        $.sequence_expression,
        $._expression,
      ),
    )),

    string_literal: $ => token(
      seq(
        '"',
        repeat(/[^\\"]/),
        '"'
      )),
  },

};

module.exports = grammar(grammarObject);
