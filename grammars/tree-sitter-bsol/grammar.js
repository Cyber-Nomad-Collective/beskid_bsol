module.exports = grammar({
	name: "bsol",

	extras: ($) => [/\s/, $.comment],

	rules: {
		source_file: ($) => repeat($.block),

		block: ($) =>
			seq(
				optional($.attribute_list),
				$.block_kind,
				optional($.string),
				choice(
					seq($.schemaless, "{", optional($.schemaless_body), "}"),
					seq("{", repeat($._block_item), "}"),
				),
			),

		attribute_list: ($) => repeat1($.attribute),

		attribute: ($) => seq("@", $.identifier, optional($.attribute_args)),

		attribute_args: ($) => seq("(", $.attribute_arg, repeat(seq(",", $.attribute_arg)), ")"),

		attribute_arg: ($) => seq($.identifier, "=", $.value),

		schemaless: (_$) => "@schemaless",

		schemaless_body: ($) => repeat1(choice($.schemaless_text, $.schemaless_braced)),

		schemaless_braced: ($) => seq("{", optional($.schemaless_body), "}"),

		schemaless_text: (_$) => token(prec(-1, /[^{}]+/)),

		_block_item: ($) => choice($.assignment, $.block),

		assignment: ($) => seq(optional($.attribute_list), $.identifier, "=", $.value),

		value: ($) =>
			choice(
				$.inline_map,
				$.ref_literal,
				$.bool_literal,
				$.list,
				$.string,
				$.type_expression,
				$.identifier,
				$.decimal_unsigned,
			),

		inline_map: ($) => seq("{", $.map_entry, repeat(seq(",", $.map_entry)), optional(","), "}"),

		map_entry: ($) => seq($.identifier, "=", $.value),

		ref_literal: ($) => seq("@", optional(seq($.identifier, "/")), $.identifier),

		bool_literal: (_$) => choice("true", "false"),

		list: ($) => seq("[", optional($.list_content), "]"),

		list_content: ($) => seq($.list_item, repeat(seq(",", $.list_item)), optional(",")),

		list_item: ($) =>
			choice(
				$.inline_map,
				$.ref_literal,
				$.bool_literal,
				$.inline_block,
				"default",
				$.string,
				$.identifier,
			),

		inline_block: ($) => seq($.block_kind, optional($.string), "{", repeat($._block_item), "}"),

		type_expression: ($) => choice($.bracket_type, $.generic_type),

		bracket_type: ($) =>
			seq($.identifier, "[", $.type_union, optional(seq(",", $.type_atom)), "]"),

		generic_type: ($) => seq($.identifier, "(", $.identifier, ")"),

		type_union: ($) => seq($.type_atom, repeat(seq("|", $.type_atom))),

		type_atom: ($) => choice($.bracket_type, $.generic_type, $.identifier),

		block_kind: ($) => $.identifier,

		identifier: (_$) => /[A-Za-z_][A-Za-z0-9_]*/,

		decimal_unsigned: (_$) => /[0-9]+/,

		string: (_$) => /"[^"]*"/,

		comment: (_$) => token(choice(seq("//", /.*/), seq("#", /.*/))),
	},
});
