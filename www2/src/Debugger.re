module Registers = {
  [@react.component]
  let make = (~gameboy: Libation.t) => {
    open React;
    open Utils;

    let makeRegister = (~key, bits, name, value) => {
      let padding = bits === 16 ? 5 : 2;
      let toHexPadding = bits === 16 ? 4 : 2;
      let dec = toHex(value, toHexPadding)->string;
      let hex = toPaddedHex(value, padding)->string;
      let bin = toPaddedBinary(value, bits)->string;

      <tr key>
        <td> name->string </td>
        <td> dec </td>
        <td> hex </td>
        <td> bin </td>
      </tr>;
    };

    let registers =
      [
        ("a", 8, Libation.getA),
        ("b", 8, Libation.getB),
        ("c", 8, Libation.getC),
        ("d", 8, Libation.getD),
        ("e", 8, Libation.getE),
        ("h", 8, Libation.getH),
        ("l", 8, Libation.getL),
        ("sp", 16, Libation.getSP),
        ("pc", 16, Libation.getPC),
      ]
      ->Belt.List.mapWithIndex((idx, (name, bits, fn)) => {
          let registerValue = fn(gameboy);
          makeRegister(~key=string_of_int(idx), bits, name, registerValue);
        })
      ->Array.of_list;

    <ToolPanel name="Registers">
      <table>
        <thead>
          <tr>
            <th> {string("Reg")} </th>
            <th> {string("Hex")} </th>
            <th> {string("Dec")} </th>
          </tr>
        </thead>
        <tbody> registers->array </tbody>
      </table>
    </ToolPanel>;
  };
};

module Breakpoints = {
  [@react.component]
  let make = (~gameboy: Libation.t) => {
    open React;
    open Utils;

    let breakpoints = gameboy->Libation.getBreakPoints;

    Js.log2("Breakpoints: ", breakpoints);

    let breakpoints =
      breakpoints
      ->Belt.Array.map(address => <li> {string(toHex(address, 6))} </li>);

    <ToolPanel name="PC Breakpoints">
      <ul>
        <li>
          <button onClick={_ => gameboy->Libation.setBreakPoint(0x2000)}>
            {string("Toggle breakpoint")}
          </button>
        </li>
        breakpoints->array
      </ul>
    </ToolPanel>;
  };
};
