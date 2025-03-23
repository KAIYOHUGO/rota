# ROTA 💫

A simple tablet/laptop mode config tool for Linux, written in rust

## Feature

- Switch mode detection (useful for 2-in-1 laptop)
- Touchreen calibration
- iio-sensor-proxy rotation
- event driven

## Get start


## Config format

There is only 3 top level blocks: `settings`, `varibles`, `actions`

settings block is for config init state.

```kdl
settings {
  default-mode {{"laptop"/"tablet"}}
  switch {{path to swicth (optional)}}
  touchscreen {{path to touchscreen (optional)}}
}
```

varibles block is for setting varible, any string prefix with `@{{name}}` will be replace with correspond value.

```kdl
variables {
  {{name}} {{value}}
}
```

actions block is for listen state change

```kdl
actions {
  {{event}} {
    {{task}}
    {{task}}
    ...
  }
}
```

the builtin event list below.

- `on-mode-laptop`
- `on-mode-tablet`
- `on-rotate-normal`
- `on-rotate-left-up`
- `on-rotate-right-up`
- `on-rotate-bottom-up`

task type list below

- `cmd {{arg}} {{arg}} ...` run command
- `action {{action name}}` run other action
- `rotation {{"enable"/"disable"}}` set should rotation or not (`on-rotate-*`)
- `rotate-calibration {{"normal"/"rotate90"/"rotate180"/"rotate270"}}` set calibration matrix on touchscreen

