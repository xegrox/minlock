# minlock

![crates.io](https://img.shields.io/crates/v/minlock.svg)

Minimal lockscreen for Wayland compositors implementing the ext-session-lock-v1 protocol.

<img src="Screenshot.png?raw=true"/>

## Installation

Via [Cargo](https://github.com/rust-lang/cargo)

    cargo install minlock

## CLI Options

```
> minlock -h
Minimal lockscreen for Wayland

Usage: minlock [OPTIONS]

Options:
      --bg-color <color>                                
      --clock-color <color>                             
      --clock-font <font>                               
      --clock-font-size <size>                          
      --indicator-idle-color <color>                    
      --indicator-wrong-color <color>                   
      --indicator-clear-color <color>                   
      --indicator-verifying-color <color>               
      --indicator-input-cursor-color <color>            
      --indicator-input-cursor-increment-color <color>  
      --indicator-input-trail-color <color>             
      --indicator-input-trail-increment-color <color>   
  -h, --help                                            Print help
  -V, --version                                         Print version

All <color> options are in RRGGBB format

```