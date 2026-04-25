# VBR UI Runtime Specification

## Overview

VBR applications use SVG as their UI format. A single SVG file describes the complete
UI of an application — its layout, widgets, panes, and interaction hints. The VBR UI
runtime reads this file and brings it to life: rendering it to a pixel buffer, handling
mouse and keyboard events, and managing pane visibility.

This document specifies the SVG format, the `vbr:` attribute vocabulary, and the
runtime behaviour required to implement it.

---

## Design Principles

- The SVG file is the single source of truth for the UI. No separate config, no
  separate schema.
- All interaction hints are embedded in the SVG as `vbr:` namespaced attributes.
  Renderers that do not understand them (browsers, Inkscape, etc.) ignore them safely.
- The runtime is intentionally thin. It does not implement a general-purpose UI toolkit.
  It implements exactly what VBR forms need and nothing more.
- Performance is a first-class concern. Only dirty rectangles are re-rendered on state
  change. Full re-renders happen only on pane transitions.

---

## SVG File Structure

Every VBR UI file must declare the `vbr` namespace and contain exactly five `<g>`
section elements, one per pane. Panes may be empty but must be present.

```xml
<svg xmlns="http://www.w3.org/2000/svg"
     xmlns:vbr="http://vbr.dev/ui"
     width="390" height="844">

  <g id="vbr-main"   vbr:section="main"   vbr:show="visible">  ... </g>
  <g id="vbr-left"   vbr:section="left"   vbr:show="hidden" vbr:width="240">  ... </g>
  <g id="vbr-right"  vbr:section="right"  vbr:show="hidden" vbr:width="240">  ... </g>
  <g id="vbr-top"    vbr:section="top"    vbr:show="hidden" vbr:height="200"> ... </g>
  <g id="vbr-bottom" vbr:section="bottom" vbr:show="hidden" vbr:height="420"> ... </g>

</svg>
```

The order of sections in the file defines the render order. Main is always rendered
first. The active side or edge pane is rendered on top.

---

## Pane System

### The Five Panes

| Pane     | Position         | Direction       | Default    |
|----------|------------------|-----------------|------------|
| `main`   | Full screen      | —               | visible    |
| `left`   | Left edge        | Slides from left  | hidden   |
| `right`  | Right edge       | Slides from right | hidden   |
| `top`    | Top edge         | Drops from top    | hidden   |
| `bottom` | Bottom edge      | Rises from bottom | hidden   |

Only one side pane may be visible at a time. Showing a second side pane implicitly
hides the first.

### Pane Attributes

| Attribute       | Applies to          | Description                                      |
|-----------------|---------------------|--------------------------------------------------|
| `vbr:section`   | `<g>`               | Declares pane identity. One of: main, left, right, top, bottom |
| `vbr:show`      | `<g>`               | Initial visibility. One of: `visible`, `hidden`  |
| `vbr:width`     | left, right `<g>`   | Panel width in pixels                            |
| `vbr:height`    | top, bottom `<g>`   | Panel height in pixels                           |

### Display Modes

When showing a side pane, the caller specifies the display mode:

**Overlay** — the pane appears on top of main. Main does not move. A semi-transparent
scrim is drawn between main and the pane.

**Nudge** — main is translated away to make room. Both main and the pane are fully
visible simultaneously. No scrim.

The nudge offset is read from the main pane's corresponding attribute:

| Side pane | Nudge attribute on main  |
|-----------|--------------------------|
| left      | `vbr:nudge-left`         |
| right     | `vbr:nudge-right`        |
| top       | `vbr:nudge-top`          |
| bottom    | `vbr:nudge-bottom`       |

The value is the number of pixels main is translated. It should match the pane's
`vbr:width` or `vbr:height`.

Top and bottom panes always use Overlay mode. Only left and right support Nudge.

---

## Widget System

Widgets are SVG elements (typically `<g>` groups) decorated with `vbr:` attributes.
The runtime identifies widgets by the presence of `vbr:type`.

### Widget Types

| `vbr:type`  | Description                                      |
|-------------|--------------------------------------------------|
| `button`    | Clickable region. Supports hover and press states |
| `toggle`    | Two-state button. On or off                      |
| `input`     | Text input field                                 |
| `label`     | Static text. No interaction                      |
| `image`     | Static image                                     |
| `checkbox`  | Boolean tick box                                 |
| `radio`     | Member of a mutually exclusive group             |
| `dismiss`   | Invisible hit region. Hides parent pane on click |

### Common Widget Attributes

| Attribute              | Description                                              |
|------------------------|----------------------------------------------------------|
| `vbr:type`             | Widget type (see above)                                  |
| `vbr:action`           | Named action to fire on click (see Actions)              |
| `vbr:fill-id`          | ID of the `<rect>` whose `fill` attribute is swapped on state change |
| `vbr:hover-fill`       | Fill colour when mouse is over the widget                |
| `vbr:mousedown-fill`   | Fill colour when mouse button is held down               |
| `vbr:disabled-fill`    | Fill colour when widget is disabled                      |
| `vbr:normal-fill`      | Fill colour in normal (resting) state. If absent, the original SVG fill is used |

The runtime reads `vbr:fill-id` to find the element to recolour. On state change it
swaps the `fill` attribute of that element and re-renders the dirty rectangle.

### Button State Machine

```
         mouse enters
normal ─────────────► hover
  ▲                     │
  │     mouse leaves    │ mouse down
  │◄────────────────────┘
  │
  │     mouse up (inside)  ──► fires vbr:action
pressed ◄──── mouse down (from hover)
  │
  │ mouse leaves (while down)
  ▼
normal  (action not fired)
```

### Toggle Widget

A toggle has two states: `off` and `on`. Additional fill attributes apply:

| Attribute              | Description                        |
|------------------------|------------------------------------|
| `vbr:state`            | Initial state. `on` or `off`       |
| `vbr:on-fill`          | Fill of background rect when on    |
| `vbr:off-fill`         | Fill of background rect when off   |
| `vbr:knob-id`          | ID of the knob `<circle>` or `<rect>` to translate |
| `vbr:knob-on-x`        | X position of knob when on         |
| `vbr:knob-off-x`       | X position of knob when off        |

### Checkbox Widget

| Attribute              | Description                                    |
|------------------------|------------------------------------------------|
| `vbr:state`            | Initial state. `checked` or `unchecked`        |
| `vbr:check-id`         | ID of the checkmark element to show/hide       |

### Radio Widget

| Attribute              | Description                                            |
|------------------------|--------------------------------------------------------|
| `vbr:group`            | Group name. Only one radio per group may be checked    |
| `vbr:state`            | Initial state. `checked` or `unchecked`                |
| `vbr:check-id`         | ID of the fill dot element to show/hide                |

### Input Widget

A text input is a `<g>` containing a background `<rect>` and a text `<text>` element.

| Attribute              | Description                                            |
|------------------------|--------------------------------------------------------|
| `vbr:type`             | `input`                                                |
| `vbr:text-id`          | ID of the `<text>` element that displays the value     |
| `vbr:cursor-id`        | ID of the cursor `<rect>` element                      |
| `vbr:value`            | Current string value                                   |
| `vbr:cursor-pos`       | Current cursor position (character index)              |
| `vbr:focused-stroke`   | Stroke colour of background rect when focused          |
| `vbr:normal-stroke`    | Stroke colour of background rect when unfocused        |
| `vbr:max-length`       | Maximum number of characters permitted                 |
| `vbr:tab-order`        | Numeric tab order for keyboard focus navigation        |

#### Input Behaviour

When an input receives focus:

1. Background rect stroke changes to `vbr:focused-stroke`
2. Cursor rect becomes visible
3. Cursor begins blinking at 500ms interval
4. Keyboard events are routed to this input

When an input loses focus:

1. Background rect stroke reverts to `vbr:normal-stroke`
2. Cursor rect is hidden
3. Keyboard events are no longer routed here

#### Keyboard Handling (focused input only)

| Key              | Behaviour                                              |
|------------------|--------------------------------------------------------|
| Printable char   | Insert at cursor position, advance cursor              |
| Backspace        | Delete character before cursor                         |
| Delete           | Delete character after cursor                          |
| Left arrow       | Move cursor left one character                         |
| Right arrow      | Move cursor right one character                        |
| Home             | Move cursor to start                                   |
| End              | Move cursor to end                                     |
| Tab              | Move focus to next input by `vbr:tab-order`            |
| Shift+Tab        | Move focus to previous input                           |
| Enter            | Fire `vbr:action` if present, otherwise no-op          |
| Escape           | Remove focus from input                                |

Clipboard paste (Ctrl+V / Cmd+V) inserts text at cursor position, truncated to
`vbr:max-length` if set.

---

## Actions

Actions are named strings on `vbr:action` attributes. The runtime maps action names
to behaviour. Built-in actions:

| Action name            | Behaviour                                              |
|------------------------|--------------------------------------------------------|
| `show-left-overlay`    | Show left pane in Overlay mode                         |
| `show-left-nudge`      | Show left pane in Nudge mode                           |
| `show-right-overlay`   | Show right pane in Overlay mode                        |
| `show-right-nudge`     | Show right pane in Nudge mode                          |
| `show-top-overlay`     | Show top pane in Overlay mode                          |
| `show-bottom-overlay`  | Show bottom pane in Overlay mode                       |
| `hide-left`            | Hide left pane, restore main if nudged                 |
| `hide-right`           | Hide right pane, restore main if nudged                |
| `hide-top`             | Hide top pane                                          |
| `hide-bottom`          | Hide bottom pane                                       |
| `hide-all`             | Hide all side panes                                    |

Any action name not in the above list is treated as a user-defined event and fired
to the VBR application code as a named callback.

---

## Hit Testing

On startup the runtime performs a single pass through the SVG and builds a hit table.
The hit table maps widget id → bounding box for all elements carrying `vbr:type`.

Hit testing on mouse events:

1. Check active overlay pane first (bottom, top, left, right in that priority order)
2. If no hit in overlay pane, check main pane
3. Coordinates are adjusted for any active nudge translation on main

The hit table is rebuilt whenever a pane becomes visible or hidden, since pane
transitions change which widgets are hittable.

---

## Rendering

### Startup

1. Parse SVG. Build widget table and hit table.
2. Render main pane to pixel buffer.
3. Blit to screen.

### On State Change (hover, press, toggle, input)

1. Mutate the relevant attribute in the SVG element (fill, text content, etc.)
2. Identify the dirty rectangle (bounding box of the changed element)
3. Re-render only that rectangle via resvg
4. Blit dirty rectangle to screen

### On Pane Transition (show/hide)

1. Update `vbr:show` attribute on the relevant pane `<g>`
2. If nudge: update translate transform on main `<g>`
3. Full re-render of visible panes
4. Rebuild hit table
5. Blit to screen

### Cursor Blink

A timer fires every 500ms while an input is focused. On each tick:

1. Toggle cursor rect `fill-opacity` between `1` and `0`
2. Re-render cursor dirty rectangle only
3. Blit

---

## Runtime API (VBR code interface)

The following methods are exposed to VBR application code:

```vba
' Pane control
Form.Show(section As String, mode As DisplayMode)
Form.Hide(section As String)

' Widget state
Form.GetValue(id As String) As String
Form.SetValue(id As String, value As String)
Form.SetEnabled(id As String, enabled As Boolean)
Form.SetVisible(id As String, visible As Boolean)

' Event handling
Form.OnAction(action As String, handler As Sub)

' Full reload (e.g. after dynamic SVG generation)
Form.Load(svg As String)
```

DisplayMode is an enum: `Overlay` or `Nudge`.

---

## Dynamic SVG Generation

VBR application code may generate SVG at runtime using the `SvgBuilder` class in
`vbr_stdlib`. The generated string is passed to `Form.Load()` which replaces the
current SVG entirely and re-initialises the runtime.

```vba
Dim svg As SvgBuilder = New SvgBuilder(390, 844)
svg.Button("save_btn", 10, 10, 120, 36, "Save")
svg.Label("title", 10, 60, "Customer Details")
svg.TextBox("name_input", 10, 90, 300, 32)
Form.Load(svg.Build())
```

`SvgBuilder` methods emit SVG elements with all required `vbr:` attributes already
set to sensible defaults. Applications may also pass raw SVG strings to `Form.Load()`
directly, including SVG produced by an LLM.

---

## Web Fallback

On web, the SVG file is served statically. The browser renders it natively. A thin
JavaScript shim reads the same `vbr:` attributes and implements the same pane
show/hide and widget state logic using DOM manipulation rather than pixel buffer
operations.

The SVG format is identical. No changes to the SVG file are needed to run on web
versus desktop.

---

## Out of Scope (V0)

The following are explicitly deferred and will not be implemented in V0:

- Animations or transitions on pane show/hide
- Drag and drop
- Sliders
- Scrollable content within a pane (OS scroll events pan the viewport)
- Custom scrollbars
- IME support for non-Latin text input
- Text selection within input fields
- Multiple simultaneous visible side panes
- Nested panes or sub-forms
- Accessibility / screen reader support

---

## File Naming Convention

VBR UI files use the extension `.vbrui`. They are valid SVG files and may be opened
in any SVG viewer. The `.vbrui` extension signals to tooling that the file contains
`vbr:` interaction attributes intended for the VBR runtime.
