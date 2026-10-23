# CADCraft Host Bridge Protocol

The postMessage bridge allows a same-origin parent web application (e.g. CoreCAD) to embed CADCraft in an `<iframe>` via `?host=parent`.

Communication happens via window `postMessage` calls between the framing page and CADCraft. Only messages from the same origin matching the embedding parent window are accepted.

## Messages from Host (Parent to CADCraft)

### `cadcraft:open`
Opens a drawing from an ArrayBuffer or Uint8Array.

```json
{
  "type": "cadcraft:open",
  "name": "Drawing.dwg",
  "bytes": "<ArrayBuffer>"
}
```

If a document with the same name is already open in a tab, CADCraft switches to that tab instead of opening a duplicate.

### `cadcraft:recent`
Populates the "Recent" section on the Start page with the user's drawings.

```json
{
  "type": "cadcraft:recent",
  "items": [
    {
      "id": "file_123",
      "name": "House_Plan.dwg",
      "updated": "2026-10-23T01:28:00Z",
      "size": 1048576
    }
  ]
}
```

- `id`: Opaque string identifier used by the host to retrieve the file.
- `name`: Display name shown in the Start page list.
- `updated`: Optional ISO 8601 timestamp string shown under the file name.
- `size`: Optional size in bytes shown alongside the date.

### `cadcraft:lisp`
Evaluates AutoLISP source code in the active drawing session.

```json
{
  "type": "cadcraft:lisp",
  "name": "script.lsp",
  "text": "(princ \"hello\")"
}
```

## Messages to Host (CADCraft to Parent)

### `cadcraft:ready`
Posted when CADCraft web runner has initialized and is listening for messages.

```json
{
  "type": "cadcraft:ready",
  "version": "0.10.0"
}
```

### `cadcraft:request-open`
Posted when the user clicks an item in the "Recent" section of the Start page.

```json
{
  "type": "cadcraft:request-open",
  "id": "file_123"
}
```

The host should respond by fetching the file and sending `cadcraft:open`.

### `cadcraft:active`
Posted whenever the active document tab changes.

```json
{
  "type": "cadcraft:active",
  "name": "House_Plan.dwg"
}
```

When the user switches to the Start tab (or no documents are open), `name` is sent as an empty string `""`.

### `cadcraft:save`
Posted when the user invokes SAVE, SAVEAS, or file export commands.

```json
{
  "type": "cadcraft:save",
  "name": "House_Plan.dwg",
  "bytes": "<ArrayBuffer>",
  "label": "Optional label",
  "source": "Optional source"
}
```

The bytes buffer is transferred with the message. The parent application is responsible for saving the file to persistent storage.

### `cadcraft:dirty`
Posted whenever unsaved modifications appear or disappear.

```json
{
  "type": "cadcraft:dirty",
  "dirty": true
}
```

### `cadcraft:checkpoint`
Posted when a version checkpoint is requested.

```json
{
  "type": "cadcraft:checkpoint",
  "label": "Checkpoint name"
}
```

### `cadcraft:failed`
Posted if CADCraft fails during startup (e.g. missing WebGPU / WebGL2 canvas support).

```json
{
  "type": "cadcraft:failed",
  "error": "Reason"
}
```
