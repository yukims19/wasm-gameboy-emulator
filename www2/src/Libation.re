type t;
[@bs.module "wasm-gameboy-emulator/wasm_gameboy_emulator"]
external bootRust: unit => t = "init";

[@bs.module "wasm-gameboy-emulator/wasm_gameboy_emulator"]
[@bs.scope "Canvases"]
external createCanvases: unit => t = "new";
[@bs.send] external drawObj: (t, t) => unit = "draw_obj";

[@bs.module "wasm-gameboy-emulator/wasm_gameboy_emulator"]
[@bs.scope "Gameboy"]
external createGameboy: unit => t = "new";

[@bs.send]
external executeOpcodesNoStop: (t, int) => unit = "execute_opcodes_no_stop";
[@bs.send] external joypadKeyPressed: (t, int) => unit = "joypad_key_pressed";
[@bs.send]
external joypadKeyReleased: (t, int) => unit = "joypad_key_released";

[@bs.send] external start: t => unit = "start_running";
[@bs.send] external stop: t => unit = "stop_running";
[@bs.send] external isRunning: t => unit = "is_running";
[@bs.send] external isVblank: t => unit = "is_vblank";
[@bs.send] external pc: t => int = "get_pc";

[@bs.send] external getA: t => int = "get_a";
[@bs.send] external getB: t => int = "get_b";
[@bs.send] external getC: t => int = "get_c";
[@bs.send] external getD: t => int = "get_d";
[@bs.send] external getE: t => int = "get_e";
[@bs.send] external getH: t => int = "get_h";
[@bs.send] external getL: t => int = "get_l";
[@bs.send] external getSP: t => int = "get_sp";
[@bs.send] external getPC: t => int = "get_pc";
[@bs.send] external flagZ: t => bool = "get_flag_z";
[@bs.send] external flagN: t => bool = "get_flag_n";
[@bs.send] external flagH: t => bool = "get_flag_h";
[@bs.send] external flagC: t => bool = "get_flag_c";
[@bs.send] external flagIME: t => bool = "get_flag_ime";
[@bs.send]
external getInterruptEnabledVblank: t => bool = "get_interrupt_enabled_vblank";
[@bs.send]
external getInterruptEnabledLcd: t => bool = "get_interrupt_enabled_lcd";
[@bs.send]
external getInterruptEnabledTimer: t => bool = "get_interrupt_enabled_timer";
[@bs.send]
external getInterruptEnabledSerial: t => bool = "get_interrupt_enabled_serial";
[@bs.send]
external getInterruptEnabledJoypad: t => bool = "get_interrupt_enabled_joypad";
[@bs.send] external cpuPaused: t => bool = "cpu_paused";
[@bs.send] external totalCycle: t => int = "total_cycle";
[@bs.send] external vramCycle: t => int = "vram_cycle";
[@bs.send] external ly: t => int = "ly";
[@bs.send] external timer: t => int = "timer";
[@bs.send] external cpuClock: t => int = "cpu_clock";
[@bs.send] external getBreakPoints: t => array(int) = "get_break_points";
[@bs.send] external setBreakPoint: (t, int) => unit = "set_break_point";
[@bs.send] external removeBreakPoint: (t, int) => unit = "remove_break_point";
[@bs.send] external toggleIsRunning: t => t = "toggle_is_running";

[@bs.module "wasm-gameboy-emulator/wasm_gameboy_emulator"]
external initPanicHook: unit => unit = "init_panic_hook";

module Memory = {
  type t;
  [@bs.module "wasm-gameboy-emulator/wasm_gameboy_emulator_bg"]
  external memory: t = "memory";

  type buffer;

  [@bs.get] external buffer: t => buffer = "buffer";
};

let saveGb: (string, t) => unit = [%raw
  {|
  function(a, b) {
    window[a] = b;
  }
|}
];
