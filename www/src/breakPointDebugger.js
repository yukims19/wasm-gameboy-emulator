import {opcode_name} from 'wasm-gameboy-emulator/wasm_gameboy_emulator';
import {compareUint8Array, interestingRanges, toHex} from './utils.js';
import React, {useState} from 'react';

const BreakPointDebugger = props => {
  const initialBreakPoints = ['00f1', '008b'];
  const {setBreakPoint, removeBreakPoint} = props;
  const [breakPoints, setBreakPoints] = useState(initialBreakPoints);
  const [newPoint, setNewPoint] = useState('');

  const handleClick = target => {
    if (target.checked) {
      setBreakPoint(parseInt(target.value, 16));
    } else {
      removeBreakPoint(parseInt(target.value, 16));
    }
  };

  const breakPointsChecker = points => {
    return points.map((point, idx) => {
      return (
        <div key={idx}>
          <input
            type="checkbox"
            value={point}
            name={point}
            onClick={event => handleClick(event.target)}
          />
          <label htmlFor={point}>0x{point}</label>
        </div>
      );
    });
  };

  const handleKeyPress = e => {
    const points = breakPoints.slice();
    if (e.key === 'Enter' && newPoint != '' && !points.includes(newPoint)) {
      points.push(newPoint);
      setBreakPoints(points);
      setNewPoint('');
    }
  };

  return (
    <div className="break-point-wrapper">
      <h3>Break Points</h3>
      <input
        placeholder="New Break Point"
        value={newPoint}
        onKeyPress={event => handleKeyPress(event)}
        onChange={event => setNewPoint(event.target.value)}
      />
      {breakPointsChecker(breakPoints)}
    </div>
  );
};
export {BreakPointDebugger};
