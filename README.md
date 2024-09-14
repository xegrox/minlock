# minlock

Minimal lockscreen for Wayland compositors implementing the ext-session-lock-v1 protocol.

<img src="Screenshot.png?raw=true"/>

## Installation

Via [Cargo](https://github.com/rust-lang/cargo)

    cargo install minlock

## CLI Options

```shell
> minlock -h
Minimal lockscreen for Wayland

Usage: minlock [OPTIONS]

Options:
  -b, --bg-color <COLOR>                                
  -c, --clock-color <COLOR>                             
      --indicator-idle-color <COLOR>                    
      --indicator-wrong-color <COLOR>                   
      --indicator-clear-color <COLOR>                   
      --indicator-verifying-color <COLOR>               
      --indicator-input-cursor-color <COLOR>            
      --indicator-input-cursor-increment-color <COLOR>  
      --indicator-input-trail-color <COLOR>             
      --indicator-input-trail-increment-color <COLOR>   
  -h, --help                                            Print help
  -V, --version                                         Print version

All <COLOR> options are in RRGGBB format

```