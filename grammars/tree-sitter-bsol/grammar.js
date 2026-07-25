module.exports = grammar({
	name: "bsol",

	extras: ($) => [/\s/, $.comment],

	rules: {
		source_file: ($) => repeat($.block),

		block: ($) =>
			seq(
				$.block_kind,
				optional($.string),
				optional("@schemaless"),
				"{",
				repeat($._block_item),
				"}",
			),

		_block_item: ($) => choice($.assignment, $.block),

		assignment: ($) => seq($.identifier, "=", $.value),

		value: ($) => choice($.string, $.identifier, $.list),

		list: ($) => seq("[", optional($.list_content), "]"),

		list_content: ($) => seq($.list_item, repeat(seq(",", $.list_item))),

		list_item: ($) => choice("default", $.string, $.identifier),

		block_kind: ($) => $.identifier,

		identifier: (_$) => /[A-Za-z_][A-Za-z0-9_]*/,

		string: (_$) => /"[^"]*"/,

		comment: (_$) => token(choice(seq("//", /.*/), seq("#", /.*/))),
	},
});
