# ROTA 💫

A simple DE agnostic tablet/laptop mode config tool for Linux, written in rust

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

## Install

```bash
cargo install https://github.com/KAIYOHUGO/rota.git
```

## Usage

```bash
rota {{path to config file}}
```

## Nix

First add rota to flake inputs.

```nix
inputs = {
  rota = {
    url = "github:kaiyohugo/rota";
    inputs.nixpkgs.follows = "nixpkgs";
  };
}
```

Then add nixosModules to nixosConfigurations

```nix
outputs =
  {
    rota, # add this
  }@inputs: {
  # ...
  
  nixosConfigurations."your host name" = nixpkgs.lib.nixosSystem rec {
    system = "your system arch";
    specialArgs = { inherit inputs outputs merge; };
    modules = [
      rota.nixosModules.${system} # add this

      # ...
    ];
  };
};
```

Finally, add rota with `service.rota`

```nix
# for cosmic de
services.rota = {
  enable = true;
  debug = false; # optional, default to false
  packages = with pkgs; [ cosmic-randr ]; # optional, default to []
  enviroment = { # optional, default to {}
    XDG_RUNTIME_DIR = "/run/user/1000";
    WAYLAND_DISPLAY = "wayland-1";
  };
  config = ''
    settings {
      default-mode "laptop"
      switch "/dev/input/event3"
    }

    // ...
  '';
};
```

## Example

The example config file and systemd service is in `config/` folder
