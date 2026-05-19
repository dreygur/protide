; JSON body → highlight as JSON
((json_body) @injection.content
  (#set! injection.language "json"))

; XML body → highlight as XML
((xml_body) @injection.content
  (#set! injection.language "xml"))

; GraphQL body → highlight as GraphQL
((graphql_data) @injection.content
  (#set! injection.language "graphql"))

; Script blocks (< {% ... %}) → highlight as JavaScript
((script) @injection.content
  (#set! injection.language "javascript"))

; Test blocks (# @tests followed by raw body) → highlight as JavaScript
((comment
  name: (_) @_name
  (#eq? @_name "tests"))
  .
  (raw_body) @injection.content
  (#set! injection.language "javascript"))
