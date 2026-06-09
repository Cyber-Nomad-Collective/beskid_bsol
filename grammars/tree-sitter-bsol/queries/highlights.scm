(block
  kind: (identifier)
  label: (string)?
  body: (block_body))

(block_body
  "{" (_block_item)* "}")

(assignment
  key: (identifier)
  "="
  value: (value))

(string) @string
(identifier) @property
(comment) @comment
