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

export const playSquare = (audioCtx, square1) => {
  console.log("sq");
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

  console.log(
    ">>>>",
    sweep_time,
    is_sweep_increase,
    sweep_shift_num,
    wave_duty_pct,
    sound_length_sec,
    volume,
    is_envelop_increase,
    envelop_shift_num,
    fr,
    frequency,
    is_restart,
    is_use_length
  );
  const osc1 = audioCtx.createOscillator();
  osc1.type = "square";

  osc1.frequency.value = frequency * 8;
  let sweepEnv = audioCtx.createGain();
  let currentTime = audioCtx.currentTime;
  sweepEnv.gain.cancelScheduledValues(currentTime);
  sweepEnv.gain.setValueAtTime(volume, currentTime);
  while (volume >= 0 && volume <= 1.5) {
    sweepEnv.gain.setValueAtTime(
      volume,
      (currentTime = currentTime + (1 / 64) * envelop_shift_num)
    );
    volume = is_envelop_increase ? volume + 0.1 : volume - 0.1;
  }

  //TODO: need to implement sweep_shifts
  osc1.connect(sweepEnv).connect(audioCtx.destination);
  osc1.start(audioCtx.currentTime);
  osc1.stop(audioCtx.currentTime + 1);
};

/* export const playSquare1 = (audioCtx, square1, f) => {
 *   console.log("s1");
 *   let soundLength = 0.045;
 *   const osc1 = audioCtx.createOscillator();
 *   osc1.type = "square";
 *   console.log(square1.fr(), square1.frequency());
 *   osc1.frequency.value = f * 8; //square1.frequency() * 10;
 *   let sweepEnv = audioCtx.createGain();
 *   sweepEnv.gain.cancelScheduledValues(audioCtx.currentTime);
 *   sweepEnv.gain.setValueAtTime(0, audioCtx.currentTime);
 *   sweepEnv.gain.linearRampToValueAtTime(0.2, audioCtx.currentTime + 1);
 *   sweepEnv.gain.linearRampToValueAtTime(
 *     0.15,
 *     audioCtx.currentTime + 1 + 3 * (1 / 64)
 *   );
 *   sweepEnv.gain.linearRampToValueAtTime(
 *     0.15,
 *     audioCtx.currentTime + 1 + 3 * (1 / 64) * 2
 *   );
 *   sweepEnv.gain.linearRampToValueAtTime(
 *     0,
 *     audioCtx.currentTime + 1 + 3 * (1 / 64) * 3
 *   );
 *   osc1.connect(sweepEnv).connect(audioCtx.destination);
 *   osc1.start(audioCtx.currentTime + 1);
 *   osc1.stop(audioCtx.currentTime + 1 + 3 * (1 / 64) * 2);
 * };
 *
 * export const playSquare2 = (audioCtx, square1, f) => {
 *   console.log("s2");
 *   const osc2 = audioCtx.createOscillator();
 *   osc2.type = "square";
 *   console.log(square1.fr(), square1.frequency());
 *   osc2.frequency.value = f * 8; //square1.frequency() * 10;
 *   const secondFStartT = audioCtx.currentTime + 1 + 3 * (1 / 64) * 2;
 *   let sweepEnv2 = audioCtx.createGain();
 *   sweepEnv2.gain.cancelScheduledValues(secondFStartT);
 *   sweepEnv2.gain.setValueAtTime(1.5, secondFStartT);
 *
 *   var volumn = 1.5;
 *   var envelopStep = 3;
 *
 *   console.log("while");
 *   while (volumn >= 0 && volumn <= 1.5) {
 *     sweepEnv2.gain.setValueAtTime(
 *       volumn,
 *       secondFStartT + (1 / 64) * envelopStep
 *     );
 *     volumn = volumn - 0.1;
 *   }
 *
 *   osc2.connect(sweepEnv2).connect(audioCtx.destination);
 *   osc2.start(secondFStartT);
 *   osc2.stop(secondFStartT + 3 * (1 / 64) * 15);
 * }; */
