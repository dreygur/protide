# API Dash - Completed Tasks

## Session: Request Panel Interactivity

- [x] Add clickable checkbox toggles for headers
- [x] Add ability to add/remove header rows
- [x] Show loading state on Send button
- [x] Make header key/value inputs editable
- [x] Make body editor editable
- [x] Add query params editing (with add/remove/toggle)

## Session: Full Text Editing Support

- [x] Unified text editing system (EditTarget enum)
- [x] Cursor positioning with click
- [x] Text selection with click and drag
- [x] Keyboard navigation (arrow keys, home, end)
- [x] Selection extension (Shift+Arrow)
- [x] Copy/Cut/Paste (Ctrl+C/X/V)
- [x] Select All (Ctrl+A)
- [x] Backspace/Delete with selection support
- [x] Tab/Enter to move between fields in headers/params

## Session: Auth Tab Implementation

- [x] Auth type selector (None, Bearer, Basic, API Key)
- [x] Bearer token input field
- [x] Basic auth username/password fields
- [x] API Key name/value/location fields
- [x] Integrate auth with HTTP request

## Session: URL <-> Params Sync

- [x] Parse URL query params on URL change
- [x] Update URL when params change (add/remove/toggle/edit)
- [x] Bidirectional sync without infinite loops
- [x] URL encoding/decoding for special characters

## Session: Environment Variables & History

- [x] Add environment variable support
- [x] Add request history
- [x] Code cleanup (removed unused imports, suppressed dead_code warnings)

## Next Steps

- [ ] Add WebSocket support
- [ ] Add GraphQL support
- [ ] Add gRPC support
