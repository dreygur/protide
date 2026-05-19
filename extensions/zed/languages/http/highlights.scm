; HTTP methods
(method) @keyword

; Request separator ###
(request_separator) @keyword

; Request separator title
(request_separator
  value: (_) @string.special)

; URL
(request
  url: (target_url) @string.special.url)

; Header name
(header
  name: (header_entity) @property)

; Header colon and value
(header ":" @punctuation.delimiter)
(header
  value: (_) @string)

; Annotation @ and keyword name (# @name, # @protocol, ...)
(comment
  "@" @keyword
  name: (identifier) @attribute)

; Annotation value (# @name list-countries)
(comment
  value: (_) @string)

; Comments
(comment) @comment

; Variable {{name}}
(variable) @string.special

[
  "{{"
  "}}"
] @punctuation.bracket

; HTTP version
(http_version) @constant

; Response status
(status_code) @number
(status_text) @string

; Variable declaration
(variable_declaration
  name: (identifier) @variable)

(variable_declaration "=" @operator)
