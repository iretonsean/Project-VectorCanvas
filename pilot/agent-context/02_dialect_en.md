ENGLISH PHRASES -> PVC
"full-width" -> `w=fill` · "N px wide" -> `w=N` · "next to / side by side" -> `row` · "below / stacked" -> `col` · "N columns" -> `grid cols=N md.cols=1` · "stacks/collapses on phones or tablets" -> `md.dir=col` / `md.cols=1` / `md.w=fill` / `md.show=false` · "centered" -> `align=center` · "pushed to the right/end" -> `w=fill` on the node before it, or `justify=between` · "soft border" -> `border=soft` · "rounded" -> `radius=md` · "muted/gray text" -> `muted` · "big / bold heading" -> `bold size=24 heading=1` · "primary button" -> `button`, "secondary" -> `fill=none` · "comfortable spacing" -> `gap=md pad=md` · "hidden unless X" -> `show=(X)` · "only one of several views" -> `slot` · "a list of things the user can add and remove" -> `list` state + `view` · "reading width" -> a column `w=640` · "icon" -> `shape … decorative`.

EXAMPLES (unrelated to any task you will receive)
```pvc
pvc 0.1 use std@0.1 theme dark
fonts std
state { playing: bool }
card w=300 gap=md {
  text "Blue in Green" bold size=16
  text "Miles Davis" muted size=12
  row gap=sm {
    button "Play" on.press={ playing = true }
    button "Pause" fill=none on.press={ playing = false }
  }
  text "Now playing" muted size=12 show=playing
}
```
```pvc
pvc 0.1 use std@0.1 theme light
fonts std
state { vol: int = 5; muted: bool }
surface {
  col w=360 gap=md {
    text "Sound" bold size=20 heading=1
    row gap=sm align=center md.dir=col {
      button "-" name="Quieter" on.press={ when vol > 0; vol = vol - 1 }
      text @vol.label n=(vol)
      button "+" name="Louder" on.press={ when vol < 10; vol = vol + 1 }
    }
    row align=center { text @mute.label w=fill; switch name=@mute.label bind=muted }
  }
}
strings en { vol.label "Volume {n}"  mute.label "Mute" }
```
```pvc
pvc 0.1 use std@0.1 theme light
fonts std
state { songs: list(title:text) max=30; draft: text }
surface {
  col w=400 gap=md {
    text "Queue" bold size=20 heading=1
    row gap=sm align=end { field "Song" bind=draft; button "Queue it" on.press={ when not is-empty(draft); append songs { title = draft }; draft = "" } }
    view songs as=s gap=xs { row gap=sm align=center { text value=(s.title) w=fill; button "x" name="Drop song" fill=none on.press={ remove s } } }
    text @count n=(count(songs)) muted
  }
}
strings en { count "{n} queued" }
```
