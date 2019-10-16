import {
  Gameboy,
  Pixel,
  FmOsc,
  to_save_state,
  load_state,
  init as initEmulation,
  init_panic_hook,
  opcode_name,
  Canvases,
} from 'wasm-gameboy-emulator/wasm_gameboy_emulator';
import {memory} from 'wasm-gameboy-emulator/wasm_gameboy_emulator_bg';
import React, {useState} from 'react';
import {
  areTypedArraysEqual,
  compareUint8Array,
  interestingRanges,
  toHex,
} from './utils.js';
import {Debugger} from './debugControls.js';
import {SoundDebugger} from './soundDebugger.js';
import {BreakPointDebugger} from './breakPointDebugger.js';
import {MbcDebugger} from './MbcDebugger.js';
import {LcdDebugger} from './LcdDebugger.js';
import {SaveStateManager} from './saveStateManager.js';
import {square1, playSquare, playSquare1, playSquare2} from './channels.jsx';

var ReactDOM = require('react-dom');

const config = {
  PIXEL_ZOOM: 1,
};

initEmulation();
const gameboyInst = Gameboy.new();
const canvases = Canvases.new();

const playSound = gameboy => {
  // //TODO:Implement playsound
  console.log('js-playsound');
};

var domContainer = document.querySelector('#memory-viewer');
// var soundContainer = document.getElementById('sound-container');
var breakPointContainer = document.getElementById('break-point-container');
var mbcContainer = document.getElementById('mbc-container');
var lcdContainer = document.getElementById('lcd-container');
let tick = -1;
const opLogMaxLength = 16;
const opLog = [];
var isRunning = false;

var render = function render(gameboy) {
  var memoryPtr = gameboyInst.memory();
  var memoryBytes = new Uint8Array(memory.buffer, memoryPtr, 65535);
  isRunning = gameboyInst.is_running();
  tick = tick + 1;

  const next = opNum => {
    if (gameboy.is_running()) {
      const startTime = Date.now();
      gameboy.execute_opcodes_no_stop(opNum ? opNum : 15000000);
      // if (gameboy.is_vblank()) {
      //   canvases.update_char_map_canvas(gameboy);
      //   canvases.render_background_map_1_as_image_data(gameboy);
      // }
      const elapsed = Date.now() - startTime;
      setTimeout(
        () => requestAnimationFrame(() => render(gameboy, memoryBytes)),
        16.6 - elapsed,
      );
    }
  };

  var pc = gameboy.get_pc();
  const opcodeLogEntry = {
    tick: tick,
    pc: pc,
    opcode: memoryBytes[pc],
    memory: new Uint8Array(memoryBytes),
  };

  opLog.push(opcodeLogEntry);
  if (opLog.length > opLogMaxLength) {
    // Take out the oldest item with shift
    opLog.shift();
  }

  var nextPc = gameboy.get_pc();

  const onStep = () => {
    console.log('onStep');
    gameboy.start_running();
    next(100000);
    gameboy.stop_running();
  };

  const onStep1 = () => {
    console.log('onStep1');
    gameboy.start_running();
    next(1);
    gameboy.stop_running();
  };

  const registers = {
    a: gameboy.get_a(),
    b: gameboy.get_b(),
    c: gameboy.get_c(),
    d: gameboy.get_d(),
    e: gameboy.get_e(),
    h: gameboy.get_h(),
    l: gameboy.get_l(),
    sp: gameboy.get_sp(),
    pc: gameboy.get_pc(),
    flags: {
      z: gameboy.get_flag_z(),
      n: gameboy.get_flag_n(),
      h: gameboy.get_flag_h(),
      c: gameboy.get_flag_c(),
      ime: gameboy.get_flag_ime(),
      interruptEnabledVblank: gameboy.get_interrupt_enabled_vblank(),
      interruptEnabledLcd: gameboy.get_interrupt_enabled_lcd(),
      interruptEnabledTimer: gameboy.get_interrupt_enabled_timer(),
      interruptEnabledSerial: gameboy.get_interrupt_enabled_serial(),
      interruptEnabledJoypad: gameboy.get_interrupt_enabled_joypad(),
      halt: gameboy.cpu_paused(),
    },
  };

  window.gb = gameboy;

  const cycleTotal = gameboy.total_cycle();
  const vramCycleTotal = gameboy.vram_cycle();
  const ly = gameboy.ly();
  const isVblank = gameboy.is_vblank();
  const timer = gameboy.timer();
  const cpuClock = gameboy.cpu_clock();

  // A debugging handle to play with in the console
  window.fullMemory = () => memoryBytes;

  const square1 = gameboy.square1();
  let sweep_time = square1.sweep_time();
  let is_sweep_increase = square1.is_sweep_increase();
  let sweep_shift_num = square1.sweep_shift_num();
  let wave_duty_pct = square1.wave_duty_pct();
  let sound_length_sec = square1.sound_length_sec();
  let volume = square1.volume() / 10;
  let is_envelop_increase = square1.is_envelop_increase();
  let envelop_shift_num = square1.envelop_shift_num();
  let fr = square1.fr();
  let frequency = square1.frequency();
  let is_restart = square1.is_restart();
  let is_use_length = square1.is_use_length();
  let isTimerEnabled = gameboy.is_timer_enabled();
  let timerCounter = gameboy.timer_counter_memory();
  let timerCycle = gameboy.timer_cycle();
  let timerCycleToIncreaseCount = gameboy.timer_cycle_to_cpu_clock();
  let timerFrequency = gameboy.timer_frequency();

  let mbc = gameboy.get_mbc();
  let romBank = gameboy.get_rom_bank();
  let ramBank = gameboy.get_ram_bank();
  let isRamEnabled = gameboy.get_is_ram_enabled();
  let isRomEnabled = gameboy.get_is_rom_banking_enabled();

  let isLdcDisplayEnabled = gameboy.is_lcd_display_enable();
  let windowTileMapSelection = gameboy.get_window_tile_map_selection();
  let isWindowDisplayEnabled = gameboy.is_window_display_enable();
  let tileDataSelection = gameboy.get_tile_data_selection();
  let bgTileMapSelection = gameboy.get_bg_tile_map_selection();
  let objSizeSelection = gameboy.get_obj_size_selection();
  let isObjDisplayEnabled = gameboy.is_obj_display_enable();
  let isBgDisplay = gameboy.is_bg_display();

  // ReactDOM.render(
  //   React.createElement(SoundDebugger, {
  //     fullMemory: memoryBytes,
  //     sweep_time,
  //     is_sweep_increase,
  //     sweep_shift_num,
  //     wave_duty_pct,
  //     sound_length_sec,
  //     volume,
  //     is_envelop_increase,
  //     envelop_shift_num,
  //     fr,
  //     frequency,
  //     is_restart,
  //     is_use_length,
  //   }),
  //   soundContainer,
  // );

  ReactDOM.render(
    React.createElement(BreakPointDebugger, {
      setBreakPoint: point => gameboy.set_break_point(point),
      removeBreakPoint: point => gameboy.remove_break_point(point),
    }),
    breakPointContainer,
  );

  ReactDOM.render(
    React.createElement(MbcDebugger, {
      mbc,
      romBank,
      ramBank,
      isRamEnabled,
      isRomEnabled,
    }),
    mbcContainer,
  );

  ReactDOM.render(
    React.createElement(LcdDebugger, {
      isLdcDisplayEnabled,
      windowTileMapSelection,
      isWindowDisplayEnabled,
      tileDataSelection,
      bgTileMapSelection,
      objSizeSelection,
      isObjDisplayEnabled,
      isBgDisplay,
    }),
    lcdContainer,
  );

  const onTogglePlay = () => {
    gameboy.toggle_is_running();
    next();
  };

  ReactDOM.render(
    React.createElement(SaveStateManager, {
      gameboy: gameboy,
    }),
    domContainer,
  );

  ReactDOM.render(
    React.createElement(Debugger, {
      tick: tick,
      isPlaying: gameboy.is_running(),
      onStep: onStep,
      onStep1: onStep1,
      onTogglePlay,
      fullMemory: memoryBytes,
      gameboy: gameboy,
      from: 0,
      to: 240,
      pc: pc,
      nextPc: nextPc,
      opLog: opLog,
      registers: registers,
      cycleTotal: cycleTotal,
      vramCycleTotal: vramCycleTotal,
      isVblank: isVblank,
      ly: ly,
      isTimerEnabled: isTimerEnabled,
      timerCounter,
      timerCycle,
      timerCycleToIncreaseCount,
      timerFrequency,
      cpuClock,
      onDrawScreen: () => {
        console.log('on draw screen');
        canvases.draw_screen_from_memory(gameboy);
      },
      onDrawCharMap: () => {
        console.log('on draw charmap');
        canvases.update_char_map_canvas(gameboy);
      },
      onClear: () => {
        // clearContext(screenCanvas);
        // clearContext(backgroundCanvas);
      },
      onDrawBackground: () => {
        console.log('on Draw back');
        canvases.render_background_map_as_image_data(gameboy);
      },
      onPlaySound: () => {
        playSound(gameboy);
      },
      gameboy,
    }),
    domContainer,
  );

  next();
};

init_panic_hook();

// gameboyInst.start_running();
// gameboyInst.execute_opcodes_no_stop();
window.gb = gameboyInst;
requestAnimationFrame(() => render(gameboyInst));
