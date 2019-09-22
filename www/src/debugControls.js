import {opcode_name} from 'wasm-gameboy-emulator/wasm_gameboy_emulator';
import {compareUint8Array, interestingRanges, toHex} from './utils.js';
import React, {useState} from 'react';

const cellColor = (memoryIdx, pc, sp, hoveredMemoryIdx) => {
  return memoryIdx === pc
    ? 'green'
    : memoryIdx === sp
    ? 'lightblue'
    : memoryIdx === hoveredMemoryIdx
    ? 'gray'
    : 'white';
};

// memoryStart/End will be aligned to 16-bit boundaries
const HexViewer = props => {
  const [visible, setVisible] = useState(false);

  const [hoveredMemoryIdx, setHoveredMemoryIdx] = useState(0);
  const [editingMemoryIndexes, setEditingMemoryIndexes] = useState([]);
  const [memoryRange, setMemoryRange] = useState(props.range);

  const {fullMemory, name, registers} = props;

  if (!visible) {
    return <button onClick={() => setVisible(!visible)}>Toggle {name}</button>;
  }

  const [from, to] = memoryRange;

  const {sp, pc} = registers;

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
        editingMemoryIndexes.filter(idx => idx !== memoryIdx),
      );
    } else if (event.which === 27) {
      setEditingMemoryIndexes(
        editingMemoryIndexes.filter(idx => idx !== memoryIdx),
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
        memoryValue !== undefined ? toHex(memoryValue, 2) : 'UNDEFINED';
      const char =
        memoryValue > 31 && memoryValue < 127
          ? String.fromCharCode(memoryValue)
          : '.';
      asciiValues.push(
        <span
          key={memoryIdx}
          style={{
            backgroundColor: hoveredMemoryIdx === memoryIdx ? 'gray' : 'white',
          }}
          onMouseOver={() => setHoveredMemoryIdx(memoryIdx)}>
          {char}
        </span>,
      );

      const isEditing = editingMemoryIndexes.includes(memoryIdx);

      const cell = isEditing ? (
        <td
          style={{
            backgroundColor: cellColor(memoryIdx, pc, sp, hoveredMemoryIdx),
            maxWidth: '2ch',
          }}>
          <input
            style={{maxWidth: '2ch'}}
            defaultValue={toHex(memoryValueHex, 2)}
            onKeyDown={handleValueChange(memoryIdx, 16)}
          />
        </td>
      ) : (
        <td
          key={memoryIdx}
          style={{
            backgroundColor: cellColor(memoryIdx, pc, sp, hoveredMemoryIdx),
          }}
          onDoubleClick={() =>
            setEditingMemoryIndexes([memoryIdx, ...editingMemoryIndexes])
          }
          onMouseOver={() => setHoveredMemoryIdx(memoryIdx)}>
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
      <thead>
        <tr>
          <th colSpan={16} onClick={() => setVisible(!visible)}>
            {name}
          </th>
          <th colSpan={20}>
            Range:{' '}
            <input
              defaultValue={toHex(from, 4)}
              style={{maxWidth: '6ch'}}
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
              style={{maxWidth: '6ch'}}
              onKeyDown={event => {
                if (event.which === 13) {
                  const value = event.target.value;
                  const parsedValue = parseInt(value, 16);
                  setMemoryRange([
                    Math.min(parsedValue - 1, from),
                    parsedValue,
                  ]);
                }
              }}
            />
          </th>
        </tr>
      </thead>
      <thead>
        <tr>
          <th />
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

  const {registers, pc, fullMemory, gameboy} = props;
  const pcValue = fullMemory[pc];

  window.me = props;

  const makeRegister = (bits, name, value, isEditing, onEdit) => {
    const maxWidth = bits === 16 ? '4ch' : '2ch';
    const padding = bits === 16 ? 5 : 2;
    const toHexPadding = bits === 16 ? 4 : 2;

    if (isEditing) {
      const handleValueChange = base => event => {
        if (event.which === 13) {
          onEdit(parseInt(event.target.value, base));
          setEditingRegisters(
            editingRegisters.filter(editingName => editingName !== name),
          );
        } else if (event.which === 27) {
          setEditingRegisters(
            editingRegisters.filter(editingName => editingName !== name),
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
              style={{maxWidth: maxWidth}}
            />
          </td>
          <td>
            <input
              onKeyDown={handleValueChange(10)}
              defaultValue={value}
              style={{maxWidth: maxWidth}}
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
            }>
            {toHex(value, toHexPadding)}
          </td>
          <td
            onDoubleClick={() =>
              setEditingRegisters([name, ...editingRegisters])
            }>
            {value.toString().padStart(padding, '0')}
          </td>
          <td>{value.toString(2).padStart(bits, '0')}</td>
        </tr>
      );
    }
  };

  const makeFlag = (name, value, isEditing, onEdit) => {
    if (isEditing) {
      const handleValueChange = isStringInput => event => {
        if (event.which === 13) {
          const value = event.target.value;
          const booleanValue = isStringInput ? value === 'true' : value === 1;
          onEdit(booleanValue);
          setEditingFlags(
            editingFlags.filter(editingName => editingName !== name),
          );
        } else if (event.which === 27) {
          setEditingFlags(
            editingFlags.filter(editingName => editingName !== name),
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
              style={{maxWidth: '4ch'}}
            />
          </td>
          <td>
            <input
              onKeyDown={handleValueChange(false)}
              defaultValue={value ? '1' : '0'}
              style={{maxWidth: '1ch'}}
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
            {value ? '1' : '0'}
          </td>
        </tr>
      );
    }
  };

  const opcodeDesc = opcode_name(pcValue, props.gameboy);
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
          'a',
          registers.a,
          editingRegisters.includes('a'),
          newValue => gameboy.set_a(newValue),
        )}
        {makeRegister(
          8,
          'b',
          registers.b,
          editingRegisters.includes('b'),
          newValue => gameboy.set_b(newValue),
        )}
        {makeRegister(
          8,
          'c',
          registers.c,
          editingRegisters.includes('c'),
          newValue => gameboy.set_c(newValue),
        )}
        {makeRegister(
          8,
          'd',
          registers.d,
          editingRegisters.includes('d'),
          newValue => gameboy.set_d(newValue),
        )}
        {makeRegister(
          8,
          'e',
          registers.e,
          editingRegisters.includes('e'),
          newValue => gameboy.set_e(newValue),
        )}
        {makeRegister(
          8,
          'h',
          registers.h,
          editingRegisters.includes('h'),
          newValue => gameboy.set_h(newValue),
        )}
        {makeRegister(
          8,
          'l',
          registers.l,
          editingRegisters.includes('l'),
          newValue => gameboy.set_l(newValue),
        )}
        {makeRegister(
          16,
          'sp',
          registers.sp,
          editingRegisters.includes('sp'),
          newValue => gameboy.set_sp(newValue),
        )}
        {makeRegister(
          16,
          'pc',
          registers.pc,
          editingRegisters.includes('pc'),
          newValue => gameboy.set_pc(newValue),
        )}
        <tr>
          <th>Flag</th>
          <th>Set?</th>
        </tr>
        {makeFlag(
          'z',
          registers.flags.z,
          editingFlags.includes('z'),
          newValue => gameboy.set_flag_z(newValue),
        )}
        {makeFlag(
          'n',
          registers.flags.n,
          editingFlags.includes('n'),
          newValue => gameboy.set_flag_n(newValue),
        )}
        {makeFlag(
          'h',
          registers.flags.h,
          editingFlags.includes('h'),
          newValue => gameboy.set_flag_h(newValue),
        )}
        {makeFlag(
          'c',
          registers.flags.c,
          editingFlags.includes('c'),
          newValue => gameboy.set_flag_c(newValue),
        )}
        {makeFlag(
          'ime',
          registers.flags.ime,
          editingFlags.includes('ime'),
          newValue => gameboy.set_ime(newValue === 'true' || newValue === '1'),
        )}
        {makeFlag(
          'Vblank',
          registers.flags.interruptEnabledVblank,
          editingFlags.includes('Vblank'),
          newValue => gameboy.set_ime(newValue === 'true' || newValue === '1'),
        )}
        {makeFlag(
          'Lcd',
          registers.flags.interruptEnabledLcd,
          editingFlags.includes('Lcd'),
          newValue => gameboy.set_ime(newValue === 'true' || newValue === '1'),
        )}
        {makeFlag(
          'Timer',
          registers.flags.interruptEnabledTimer,
          editingFlags.includes('Timer'),
          newValue => gameboy.set_ime(newValue === 'true' || newValue === '1'),
        )}
        {makeFlag(
          'Serial',
          registers.flags.interruptEnabledSerial,
          editingFlags.includes('Serial'),
          newValue => gameboy.set_ime(newValue === 'true' || newValue === '1'),
        )}
        {makeFlag(
          'Joypad',
          registers.flags.interruptEnabledJoypad,
          editingFlags.includes('Joypad'),
          newValue => gameboy.set_ime(newValue === 'true' || newValue === '1'),
        )}
        {makeFlag(
          'Halt?',
          registers.flags.halt,
          editingFlags.includes('Halt'),
          newValue => gameboy.set_ime(newValue === 'true' || newValue === '1'),
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
    onStep1,
    onTogglePlay,
    isPlaying,
    isVblank,
    tick,
    cycleTotal,
    vramCycleTotal,
    ly,
    timer,
    cpuClock,
    onDrawScreen,
    onDrawCharMap,
    onClear,
    onDrawBackground,
    onPlaySound,
    gameboy,
  } = props;
  const pcValue = fullMemory[pc];

  const toggleButton = (
    <button onClick={onTogglePlay}>{isPlaying ? '❙❙' : '▶'}</button>
  );
  const stepButton = <button onClick={onStep}>{'>>'}</button>;
  const stepButton1 = <button onClick={onStep1}>{'>> 1'}</button>;

  const opcodeDesc = opcode_name(pcValue, gameboy);
  return (
    <table id="control-view">
      <thead>
        <tr>
          <th>
            <button onClick={onPlaySound}>Sound</button>
            <button onClick={() => onDrawScreen()}>Draw Screen</button>
            <button onClick={() => onDrawCharMap()}>Draw CharMap</button>
            {toggleButton}
            {stepButton}
            {stepButton1}
          </th>
        </tr>
        <tr>
          <th>
            <button onClick={() => onDrawBackground()}>DrawBackground</button>
            <button onClick={onClear}>Clear</button>
          </th>
        </tr>
        <tr>
          <th>Tick:</th>
          <th>{tick}</th>
        </tr>
        <tr>
          <th>Cycle Totoal:</th>
          <th>{cycleTotal}</th>
        </tr>
        <tr>
          <th>Vram Cycle Totoal:</th>
          <th>{vramCycleTotal}</th>
        </tr>
        <tr>
          <th>LY:</th>
          <th>{ly}</th>
        </tr>
        <tr>
          <th>Vblank?:</th>
          <th>{String(isVblank)}</th>
        </tr>
        <tr>
          <th>Timer:</th>
          <th>{timer}</th>
        </tr>
        <tr>
          <th>CPU Clock:</th>
          <th>{cpuClock}</th>
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
  const {opLog} = props;

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
        <td>{opcode_name(log.opcode, props.gameboy)}</td>
        <td>{idx > 0 ? <button onClick={logDiff(idx)}>Diff</button> : null}</td>
      </tr>
    );
  });

  return (
    <div style={{width: '30%', overflow: 'scroll'}}>
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
    <div style={{display: 'flex', alignContent: 'stretch', flexWrap: 'wrap'}}>
      <div
        style={{
          display: 'flex',
          alignContent: 'stretch',
          flexDirection: 'column',
        }}>
        <Controls
          fullMemory={props.fullMemory}
          onDrawScreen={props.onDrawScreen}
          onDrawCharMap={props.onDrawCharMap}
          onClear={props.onDrawCharMap}
          onDrawBackground={props.onDrawBackground}
          onPlaySound={props.onPlaySound}
          gameboy={props.gameboy}
          onStep={props.onStep}
          onStep1={props.onStep1}
          onTogglePlay={props.onTogglePlay}
          isPlaying={props.isPlaying}
          pc={props.pc}
          nextPc={props.nextPc}
          tick={props.tick}
          cycleTotal={props.cycleTotal}
          vramCycleTotal={props.vramCycleTotal}
          ly={props.ly}
          isVblank={props.isVblank}
          timer={props.timer}
          cpuClock={props.cpuClock}
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
        name={'Sound on/off'}
        fullMemory={props.fullMemory}
        range={[0xff20, 0xff30]}
        registers={props.registers}
      />

      <HexViewer
        name={'Sound Controller'}
        fullMemory={props.fullMemory}
        range={[0xff10, 0xff3f]}
        registers={props.registers}
      />

      <HexViewer
        name={'LCD'}
        fullMemory={props.fullMemory}
        range={[0xff40, 0xff50]}
        registers={props.registers}
      />

      <HexViewer
        name={'stack'}
        fullMemory={props.fullMemory}
        range={[0xffd0, 0xffff]}
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

export {Debugger};
