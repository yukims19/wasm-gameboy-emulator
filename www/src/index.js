import {
  Canvas,
  Pixel,
  init,
  opcode_name
} from "wasm-gameboy-emulator/wasm_gameboy_emulator";
import { memory } from "wasm-gameboy-emulator/wasm_gameboy_emulator_bg";
import React, { useState } from "react";
import { compareUint8Array, interestingRanges, toHex } from "./utils.js";

var ReactDOM = require("react-dom");

var PIXEL_SIZE = 5; // px

var gameboyCanvas = Canvas.new();
var width = gameboyCanvas.width();
var height = gameboyCanvas.height();

var canvas = document.getElementById("gameboy-canvas");
var tbody = document.getElementById("gameboy-table");
var pcCounter = document.getElementById("pc-counter");

canvas.height = PIXEL_SIZE * height;
canvas.width = PIXEL_SIZE * width;

var ctx = canvas.getContext("2d");

ctx.imageSmoothingEnabled = false;

var getIndex = function getIndex(row, column) {
  return row * width + column;
};

var counter = 0;

var drawTiles = function drawTiles(tile, x, y) {
  //console.log("drawTiles:", tile, x, y);
  var col = x;
  var colIdx = 0;
  var row = y;

  for (var idx = 0; idx < tile.length; idx++) {
    switch (tile[idx]) {
      case Pixel.White:
        ctx.fillStyle = "white";
        break;
      case Pixel.LightGray:
        ctx.fillStyle = "lightgray";
        break;
      case Pixel.DarkGray:
        ctx.fillStyle = "darkgray";
        break;
      case Pixel.Black:
        ctx.fillStyle = "black";
        break;
      default:
        ctx.fillStyle = "red";
    }
    if (idx !== 0 && idx % 8 === 0) {
      col = x;
      colIdx = 0;
      row = row + PIXEL_SIZE;
    }
    // console.log(
    //     "Draw: ",
    //     idx * (PIXEL_SIZE + 1) + x,
    //     idx * (PIXEL_SIZE + 1) + y,
    //     PIXEL_SIZE,
    //     PIXEL_SIZE,
    //     ctx.fillStyle
    // );
    ctx.strokeRect(col + colIdx * PIXEL_SIZE, row, PIXEL_SIZE, PIXEL_SIZE);

    if (colIdx % 8 === 0) {
      ctx.strokeRect(col + colIdx * PIXEL_SIZE, row, PIXEL_SIZE, PIXEL_SIZE);
    }

    ctx.fillRect(col + colIdx * PIXEL_SIZE, row, PIXEL_SIZE, PIXEL_SIZE);
    colIdx++;
  }
};

const getTiles = gameboy => {
  var pixelsPtr = gameboy.pixels();
  var pixels = new Uint8Array(memory.buffer, pixelsPtr, width * height);
  var tiles = [];
  var sliceIdx = 0;
  while (sliceIdx < pixels.length) {
    tiles.push(pixels.slice(sliceIdx, sliceIdx + 64));
    sliceIdx = sliceIdx + 64;
  }
  return tiles;
};

var renderCharRamTiles = function renderCharRamTiles(gameboy) {
  const tiles = getTiles(gameboy);
  let x = 0;
  let y = 0;
  tiles.forEach(function(ele, i) {
    drawTiles(ele, x, y);
    x = x + 8 * PIXEL_SIZE;
    if (x >= canvas.width) {
      x = 0;
      y = y + 8 * PIXEL_SIZE;
    } else {
    }
  });
};

var renderBackground1 = function renderBackground1(gameboy, fullMemory) {
  const tiles = getTiles(gameboy);
  const gameboyBackgroundMap1 = fullMemory.slice(0x9800, 0x9c00);

  let x = 0;
  let y = 0;
  gameboyBackgroundMap1.forEach(function(ele, idx) {
    drawTiles(tiles[ele], x, y);
    x = x + 8 * PIXEL_SIZE;
    if (x >= canvas.width) {
      //console.log("exeed");
      x = 0;
      y = y + 8 * PIXEL_SIZE;
    }
  });
};

const cellColor = (memoryIdx, pc, sp, hoveredMemoryIdx) => {
  return memoryIdx === pc
    ? "green"
    : memoryIdx === sp
    ? "lightblue"
    : memoryIdx === hoveredMemoryIdx
    ? "gray"
    : "white";
};

// memoryStart/End will be aligned to 16-bit boundaries
const HexViewer = props => {
  const [visible, setVisible] = useState(false);

  const [hoveredMemoryIdx, setHoveredMemoryIdx] = useState(0);
  const [editingMemoryIndexes, setEditingMemoryIndexes] = useState([]);
  const [memoryRange, setMemoryRange] = useState(props.range);

  const { fullMemory, name, registers } = props;

  if (!visible) {
    return <button onClick={() => setVisible(!visible)}>Toggle {name}</button>;
  }

  const [from, to] = memoryRange;

  const { sp, pc } = registers;

  const memoryStart = Math.floor(from / 16) * 16;
  const memoryEnd = Math.ceil(to / 16) * 16;

  const memory = fullMemory.slice(memoryStart, memoryEnd);

  const rowSize = 16;

  const rows = [];

  const handleValueChange = (memoryIdx, base) => event => {
    const value = event.target.value;
    const parsedValue = parseInt(value, base);

    if (event.which === 13) {
      fullMemory.set([parsedValue], memoryIdx);
      setEditingMemoryIndexes(
        editingMemoryIndexes.filter(idx => idx !== memoryIdx)
      );
    } else if (event.which === 27) {
      setEditingMemoryIndexes(
        editingMemoryIndexes.filter(idx => idx !== memoryIdx)
      );
    }
  };

  for (
    var rowIdx = memoryStart / rowSize;
    rowIdx * rowSize <= memoryEnd;
    rowIdx++
  ) {
    const cells = [<th key="addr">{toHex(rowIdx * rowSize, 4)}</th>];
    let asciiValues = [];

    for (var cellIdx = 0; cellIdx < rowSize; cellIdx++) {
      const memoryIdx = rowIdx * rowSize + cellIdx;
      const memoryValue = fullMemory[memoryIdx];
      const memoryValueHex =
        memoryValue !== undefined ? toHex(memoryValue, 2) : "UNDEFINED";
      const char =
        memoryValue > 31 && memoryValue < 127
          ? String.fromCharCode(memoryValue)
          : ".";
      asciiValues.push(
        <span
          key={memoryIdx}
          style={{
            backgroundColor: hoveredMemoryIdx === memoryIdx ? "gray" : "white"
          }}
          onMouseOver={() => setHoveredMemoryIdx(memoryIdx)}
        >
          {char}
        </span>
      );

      const isEditing = editingMemoryIndexes.includes(memoryIdx);

      const cell = isEditing ? (
        <td
          style={{
            backgroundColor: cellColor(memoryIdx, pc, sp, hoveredMemoryIdx),
            maxWidth: "2ch"
          }}
        >
          <input
            style={{ maxWidth: "2ch" }}
            defaultValue={toHex(memoryValueHex, 2)}
            onKeyDown={handleValueChange(memoryIdx, 16)}
          />
        </td>
      ) : (
        <td
          key={memoryIdx}
          style={{
            backgroundColor: cellColor(memoryIdx, pc, sp, hoveredMemoryIdx)
          }}
          onDoubleClick={() =>
            setEditingMemoryIndexes([memoryIdx, ...editingMemoryIndexes])
          }
          onMouseOver={() => setHoveredMemoryIdx(memoryIdx)}
        >
          {toHex(memoryValueHex, 2)}
        </td>
      );
      cells.push(cell);
    }

    cells.push(<td key="ascii">{asciiValues}</td>);

    const row = <tr key={rowIdx}>{cells}</tr>;

    rows.push(row);
  }

  return (
    <table id="memory-view">
      <thead onClick={() => setVisible(!visible)}>
        <tr>
          <th colSpan={16}>{name}</th>
          <th colSpan={20}>
            Range:{" "}
            <input
              defaultValue={toHex(from, 4)}
              style={{ maxWidth: "6ch" }}
              onKeyDown={event => {
                if (event.which === 13) {
                  const value = event.target.value;
                  const parsedValue = parseInt(value, 16);
                  setMemoryRange([parsedValue, Math.max(parsedValue + 1, to)]);
                }
              }}
            />
            -
            <input
              defaultValue={toHex(to, 4)}
              style={{ maxWidth: "6ch" }}
              onKeyDown={event => {
                if (event.which === 13) {
                  const value = event.target.value;
                  const parsedValue = parseInt(value, 16);
                  setMemoryRange([
                    Math.min(parsedValue - 1, from),
                    parsedValue
                  ]);
                }
              }}
            />
          </th>
        </tr>
      </thead>
      <thead>
        <tr>
          <th></th>
          <th>00</th>
          <th>11</th>
          <th>22</th>
          <th>33</th>
          <th>44</th>
          <th>55</th>
          <th>66</th>
          <th>77</th>
          <th>88</th>
          <th>99</th>
          <th>aa</th>
          <th>bb</th>
          <th>cc</th>
          <th>dd</th>
          <th>ee</th>
          <th>ff</th>
          <th>0123456789ABCDEF</th>
        </tr>
      </thead>
      <tbody id="gameboy-table">{rows}</tbody>
    </table>
  );
};

const CPUViewer = props => {
  const [editingRegisters, setEditingRegisters] = useState([]);
  const [editingFlags, setEditingFlags] = useState([]);

  const { registers, pc, fullMemory, gameboy } = props;
  const pcValue = fullMemory[pc];

  const makeRegister = (bits, name, value, isEditing, onEdit) => {
    const maxWidth = bits === 16 ? "4ch" : "2ch";
    const padding = bits === 16 ? 5 : 2;
    const toHexPadding = bits === 16 ? 4 : 2;

    if (isEditing) {
      const handleValueChange = base => event => {
        if (event.which === 13) {
          onEdit(parseInt(event.target.value, base));
          setEditingRegisters(
            editingRegisters.filter(editingName => editingName !== name)
          );
        } else if (event.which === 27) {
          setEditingRegisters(
            editingRegisters.filter(editingName => editingName !== name)
          );
        }
      };

      return (
        <tr>
          <td>{name}</td>
          <td>
            <input
              onKeyDown={handleValueChange(16)}
              defaultValue={toHex(value, toHexPadding)}
              style={{ maxWidth: maxWidth }}
            />
          </td>
          <td>
            <input
              onKeyDown={handleValueChange(10)}
              defaultValue={value}
              style={{ maxWidth: maxWidth }}
            />
          </td>
        </tr>
      );
    } else {
      return (
        <tr>
          <td>{name}</td>
          <td
            onDoubleClick={() =>
              setEditingRegisters([name, ...editingRegisters])
            }
          >
            {toHex(value, toHexPadding)}
          </td>
          <td
            onDoubleClick={() =>
              setEditingRegisters([name, ...editingRegisters])
            }
          >
            {value.toString().padStart(padding, "0")}
          </td>
          <td>{value.toString(2).padStart(bits, "0")}</td>
        </tr>
      );
    }
  };

  const makeFlag = (name, value, isEditing, onEdit) => {
    if (isEditing) {
      const handleValueChange = isStringInput => event => {
        if (event.which === 13) {
          const value = event.target.value;
          const booleanValue = isStringInput ? value === "true" : value === 1;
          onEdit(booleanValue);
          setEditingFlags(
            editingFlags.filter(editingName => editingName !== name)
          );
        } else if (event.which === 27) {
          setEditingFlags(
            editingFlags.filter(editingName => editingName !== name)
          );
        }
      };

      return (
        <tr>
          <td>{name}</td>
          <td>
            <input
              onKeyDown={handleValueChange(true)}
              defaultValue={value.toString()}
              style={{ maxWidth: "4ch" }}
            />
          </td>
          <td>
            <input
              onKeyDown={handleValueChange(false)}
              defaultValue={value ? "1" : "0"}
              style={{ maxWidth: "1ch" }}
            />
          </td>
        </tr>
      );
    } else {
      return (
        <tr>
          <td>{name}</td>
          <td onDoubleClick={() => setEditingFlags([name, ...editingFlags])}>
            {value.toString()}
          </td>
          <td onDoubleClick={() => setEditingFlags([name, ...editingFlags])}>
            {value ? "1" : "0"}
          </td>
        </tr>
      );
    }
  };

  const opcodeDesc = opcode_name(pcValue);
  return (
    <table id="cpu-view">
      <thead>
        <tr>
          <th>Reg/Loc</th>
          <th>Hex</th>
          <th>Dec</th>
        </tr>
      </thead>
      <tbody>
        {makeRegister(
          8,
          "a",
          registers.a,
          editingRegisters.includes("a"),
          newValue => gameboy.set_a(newValue)
        )}
        {makeRegister(
          8,
          "b",
          registers.b,
          editingRegisters.includes("b"),
          newValue => gameboy.set_b(newValue)
        )}
        {makeRegister(
          8,
          "c",
          registers.c,
          editingRegisters.includes("c"),
          newValue => gameboy.set_c(newValue)
        )}
        {makeRegister(
          8,
          "d",
          registers.d,
          editingRegisters.includes("d"),
          newValue => gameboy.set_d(newValue)
        )}
        {makeRegister(
          8,
          "e",
          registers.e,
          editingRegisters.includes("e"),
          newValue => gameboy.set_e(newValue)
        )}
        {makeRegister(
          8,
          "h",
          registers.h,
          editingRegisters.includes("h"),
          newValue => gameboy.set_h(newValue)
        )}
        {makeRegister(
          8,
          "l",
          registers.l,
          editingRegisters.includes("l"),
          newValue => gameboy.set_l(newValue)
        )}
        {makeRegister(
          16,
          "sp",
          registers.sp,
          editingRegisters.includes("sp"),
          newValue => gameboy.set_sp(newValue)
        )}
        {makeRegister(
          16,
          "pc",
          registers.pc,
          editingRegisters.includes("pc"),
          newValue => gameboy.set_pc(newValue)
        )}
        <tr>
          <th>Flag</th>
          <th>Set?</th>
        </tr>
        {makeFlag(
          "z",
          registers.flags.z,
          editingFlags.includes("z"),
          newValue => gameboy.set_flag_z(newValue)
        )}
        {makeFlag(
          "n",
          registers.flags.n,
          editingFlags.includes("n"),
          newValue => gameboy.set_flag_n(newValue)
        )}
        {makeFlag(
          "h",
          registers.flags.h,
          editingFlags.includes("h"),
          newValue => gameboy.set_flag_h(newValue)
        )}
        {makeFlag(
          "c",
          registers.flags.c,
          editingFlags.includes("c"),
          newValue => gameboy.set_flag_c(newValue)
        )}
      </tbody>
    </table>
  );
};

const Controls = props => {
  const {
    pc,
    fullMemory,
    onStep,
    onTogglePlay,
    isPlaying,
    tick,
    onDraw,
    onDrawBackground
  } = props;
  const pcValue = fullMemory[pc];

  const toggleButton = (
    <button onClick={onTogglePlay}>{isPlaying ? "❙❙" : "▶"}</button>
  );
  const stepButton = <button onClick={onStep}>{">>"}</button>;

  const opcodeDesc = opcode_name(pcValue);
  return (
    <table id="control-view">
      <thead>
        <tr>
          <th>
            <button onClick={onDraw}>Draw</button>
            {toggleButton}
            {stepButton}
          </th>
        </tr>
        <tr>
          <th>
            <button onClick={onDrawBackground}>DrawBackground</button>
            {toggleButton}
            {stepButton}
          </th>
        </tr>
        <tr>
          <th>Tick:</th>
          <th>{tick}</th>
        </tr>
      </thead>
      <thead>
        <tr>
          <th>PC: {toHex(pc, 4)}</th>
          <th>PC: {toHex(pcValue, 2)}</th>
        </tr>
        <tr>
          <th colSpan={20}>
            {opcodeDesc} ({toHex(pcValue, 2)})
          </th>
        </tr>
      </thead>
    </table>
  );
};

const parseHex = hexString => parseInt(hexString, 16);

const OpLogViewer = props => {
  const { opLog } = props;

  const logDiff = idx => _event => {
    const diffs = compareUint8Array(opLog[idx].memory, opLog[idx - 1].memory);
    console.table(diffs);
  };

  const rows = opLog.map((log, idx) => {
    return (
      <tr key={log.tick}>
        <td>{log.tick}</td>
        <td>{toHex(log.pc, 4)}</td>
        <td>{toHex(log.opcode, 2)}</td>
        <td>{opcode_name(log.opcode)}</td>
        <td>{idx > 0 ? <button onClick={logDiff(idx)}>Diff</button> : null}</td>
      </tr>
    );
  });

  return (
    <div style={{ width: "30%", overflow: "scroll" }}>
      <table id="memory-view">
        <thead>
          <tr>
            <th>Tick</th>
            <th>PC</th>
            <th>opcode</th>
            <th>desc</th>
          </tr>
        </thead>
        <tbody id="gameboy-table">{rows}</tbody>
      </table>
    </div>
  );
};

const Debugger = props => {
  return (
    <div style={{ display: "flex", alignContent: "stretch", flexWrap: "wrap" }}>
      <div
        style={{
          display: "flex",
          alignContent: "stretch",
          flexDirection: "column"
        }}
      >
        <Controls
          fullMemory={props.fullMemory}
          onDraw={props.onDraw}
          onDrawBackground={props.onDrawBackground}
          onStep={props.onStep}
          onTogglePlay={props.onTogglePlay}
          isPlaying={props.isPlaying}
          pc={props.pc}
          nextPc={props.nextPc}
          tick={props.tick}
        />

        <CPUViewer
          gameboy={props.gameboy}
          fullMemory={props.fullMemory}
          registers={props.registers}
          stepButton={props.stepButton}
          pc={props.nextPc}
        />
      </div>

      <OpLogViewer opLog={props.opLog} />

      <HexViewer
        name={interestingRanges.bootRom.desc}
        fullMemory={props.fullMemory}
        range={[0x00, 0x02ff]}
        registers={props.registers}
      />

      <HexViewer
        name={"nintendo-logo-from-bootrom"}
        fullMemory={props.fullMemory}
        range={[0x00a8, 0x00d7]}
        registers={props.registers}
      />

      <HexViewer
        name={"stack"}
        fullMemory={props.fullMemory}
        range={[0xfffe, 0xffff]}
        registers={props.registers}
      />

      <HexViewer
        name={interestingRanges.backgroundMap1.desc}
        fullMemory={props.fullMemory}
        range={interestingRanges.backgroundMap1.range}
        registers={props.registers}
      />

      <HexViewer
        name={interestingRanges.backgroundMap2.desc}
        fullMemory={props.fullMemory}
        range={interestingRanges.backgroundMap2.range}
        registers={props.registers}
      />
    </div>
  );
};

var domContainer = document.querySelector("#memory-viewer");

// TODO: Move this into the Rust side
let tick = -1;
let isPlaying = false;
const opLogMaxLength = 16;
const opLog = [];

var render = function render(gameboy, memoryBytes) {
  tick = tick + 1;

  const next = () => {
    gameboy.execute_opcodes(1000);
    //renderCharRamTiles(gameboy);
    requestAnimationFrame(() => render(gameboy, memoryBytes));
  };

  var pc = gameboy.get_pc();
  const opcodeLogEntry = {
    tick: tick,
    pc: pc,
    opcode: memoryBytes[pc],
    memory: new Uint8Array(memoryBytes)
  };

  opLog.push(opcodeLogEntry);
  if (opLog.length > opLogMaxLength) {
    // Take out the oldest item with shift
    opLog.shift();
  }

  var nextPc = gameboy.get_pc();

  const onStep = () => next();
  const onTogglePlay = () => {
    isPlaying = !isPlaying;
    next();
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
      c: gameboy.get_flag_c()
    }
  };

  ReactDOM.render(
    React.createElement(Debugger, {
      tick: tick,
      isPlaying: isPlaying,
      onStep: onStep,
      onTogglePlay: onTogglePlay,
      fullMemory: memoryBytes,
      gameboy: gameboy,
      from: 0,
      to: 240,
      pc: pc,
      nextPc: nextPc,
      opLog: opLog,
      registers: registers,
      onDraw: () => renderCharRamTiles(gameboy),
      onDrawBackground: () => renderBackground1(gameboy, memoryBytes)
    }),
    domContainer
  );

  if (isPlaying) {
    next();
  }
};

var memoryPtr = gameboyCanvas.memory();
var memoryBytes = new Uint8Array(memory.buffer, memoryPtr, 65535);
const [bg2start, bg2end] = interestingRanges.backgroundMap2.range;
memoryBytes.fill(0, bg2start, bg2end);

init();

// gameboyCanvas.set_pc(0x1d);
gameboyCanvas.set_a(0xff);

requestAnimationFrame(() => render(gameboyCanvas, memoryBytes));
