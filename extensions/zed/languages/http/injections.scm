; JSON body → highlight as JSON
((json_body) @injection.content
  (#set! injection.language "json"))

; XML body → highlight as XML
((xml_body) @injection.content
  (#set! injection.language "xml"))

; GraphQL body → highlight as GraphQL
((graphql_data) @injection.content
  (#set! injection.language "graphql"))
