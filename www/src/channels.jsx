export const sheepJump = audioCtx => {
  /* NR10_REG = 0x15;
   * NR11_REG = 0x96;
   * NR12_REG = 0x73;
   * NR13_REG = 0xBB;
   * NR14_REG = 0x85; */
  const sweepLength = 0.0078;
  const soundLength = 0.164;
  const osc = audioCtx.createOscillator();
  const initialFrequency = 240;
  let frequency = initialFrequency;
  const shiftOp = "add";

  osc.type = "square";
  console.log("time", audioCtx.currentTime);
  osc.frequency.setValueAtTime(frequency, audioCtx.currentTime + 1);
  frequency = freqAfterSweepShifts(frequency, 1, shiftOp);

  osc.frequency.setValueAtTime(
    frequency,
    audioCtx.currentTime + 1 + sweepLength * 1
  );
  frequency = freqAfterSweepShifts(frequency, 2, shiftOp);

  osc.frequency.setValueAtTime(
    frequency,
    audioCtx.currentTime + 1 + sweepLength * 2
  );
  frequency = freqAfterSweepShifts(frequency, 3, shiftOp);

  osc.frequency.setValueAtTime(
    frequency,
    audioCtx.currentTime + 1 + sweepLength * 3
  );
  frequency = freqAfterSweepShifts(frequency, 4, shiftOp);

  osc.frequency.setValueAtTime(
    frequency,
    audioCtx.currentTime + 1 + sweepLength * 4
  );
  frequency = freqAfterSweepShifts(frequency, 5, shiftOp);

  osc.frequency.setValueAtTime(
    frequency,
    audioCtx.currentTime + 1 + sweepLength * 5
  );

  //  osc.frequency.setValueAtTime(550, audioCtx.currentTime + 1 + sweepLength * 5);

  return osc;
};

/*
   function playSweep() {
   let soundLength = 0.045;
   //    let soundLength = 0.0078 * 5;
   let attackTime = 1;
   let releaseTime = 2;
   const osc = square1(audioCtx);
   console.log(osc);
   let sweepEnv = audioCtx.createGain();
   sweepEnv.gain.cancelScheduledValues(audioCtx.currentTime);
   sweepEnv.gain.setValueAtTime(0, audioCtx.currentTime);

   sweepEnv.gain.linearRampToValueAtTime(0.7, audioCtx.currentTime + 1);
   sweepEnv.gain.linearRampToValueAtTime(
   0.6,
   audioCtx.currentTime + 1 + 3 * (1 / 64)
   );

   sweepEnv.gain.linearRampToValueAtTime(
   0.5,

   audioCtx.currentTime + 1 + 3 * (1 / 64) * 2
   );
   sweepEnv.gain.linearRampToValueAtTime(
   0.4,
   audioCtx.currentTime + 1 + 3 * (1 / 64) * 3
   );
   osc.connect(sweepEnv).connect(audioCtx.destination);
   console.log("time", audioCtx.currentTime);
   osc.start(audioCtx.currentTime + 1);
   console.log("started", osc);
   osc.stop(audioCtx.currentTime + 1 + soundLength);
   console.log("finished", osc);
   }
 */
const freqAfterSweepShifts = (f, n, operation) => {
  let frequency;
  switch (operation) {
    case "add":
      frequency = f + f / Math.pow(2, n);
      console.log("add", n, f, frequency);
      return frequency;
    case "sub":
      frequency = f - f / Math.pow(2, n);
      console.log("sub", n, f, frequency);

      return frequency;
    default:
      throw "freqAfterSweepShits - Operation not recognized";
  }
};

export const square1 = (audioCtx, frequency) => {
  const osc = audioCtx.createOscillator();
  osc.type = "square";
  osc.frequency.value = frequency * 1.28;
  return osc;
};
