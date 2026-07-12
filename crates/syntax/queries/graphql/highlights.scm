; GraphQL highlights, authored here because the tree-sitter-graphql crate
; (joowani, 0.1.0) ships no highlight query of its own. Capture names are
; chosen to fold onto the syntax crate's coarse Token set (see CAPTURES in
; lib.rs) by longest dot-separated prefix.

(comment) @comment

(description) @string
(string_value) @string
(int_value) @number
(float_value) @number
(boolean_value) @constant
(null_value) @constant
(enum_value) @constant

; query / mutation / subscription
(operation_type) @keyword

[
  "fragment"
  "on"
  "schema"
  "scalar"
  "type"
  "interface"
  "union"
  "enum"
  "input"
  "directive"
  "extend"
  "implements"
  "repeatable"
] @keyword

(named_type (name) @type)

; The name a type definition introduces (e.g. `Query` in `type Query`).
(scalar_type_definition (name) @type)
(object_type_definition (name) @type)
(interface_type_definition (name) @type)
(union_type_definition (name) @type)
(enum_type_definition (name) @type)
(input_object_type_definition (name) @type)

(field (name) @property)
(alias (name) @property)
(argument (name) @property)
(object_field (name) @property)
(field_definition (name) @property)
(input_value_definition (name) @property)

(directive (name) @attribute)
(directive_definition (name) @attribute)

(fragment_name (name) @function)

(variable) @variable.parameter

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

(comma) @punctuation.delimiter

[
  ":"
  "!"
  "="
  "|"
  "&"
  "..."
  "@"
  "$"
] @punctuation.delimiter
