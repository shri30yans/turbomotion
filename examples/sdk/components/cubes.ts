// cube, puzzleCube and letterCube: three 3D cubes made of shape faces. Run: turbomotion sdk render cubes.ts --sheet cubes.png --sheet-only --times 0.2,1,2,3
import { comp, shot, timeline, light, cube, puzzleCube, letterCube, rect } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, bg: "#0b0d12" });
const tl = timeline();
const dot = (size: number) => [{ ellipse: [0, 0, size * 0.28, size * 0.28], fill: "#ffffff" }];

const a = cube({ id: "a", size: 150, pos: [200, 270, 0], rot: [-20, 30, 0], faces: ["#3D6BFF", { fill: "#FF5A1F", items: dot(150) }, "#C8F031"] });
tl.to(a.root, { rotation: [null, "+=300", null], duration: 3, ease: "none" }, 0);

const puzzle = puzzleCube({ id: "p", size: 180, pos: [480, 270, 0], tile: (i, s) => dot(s).map((d) => ({ ...d, fill: i % 2 ? "#0b0d12" : "#ffffff" })) });
puzzle.root.set({ rotation: [-22, 35, 0] });
tl.to(puzzle.slabs[0], { rotation: [null, 90, null], duration: 0.8, ease: "expo.inOut" }, 0.6).to(puzzle.slabs[2], { rotation: [null, -90, null], duration: 0.8, ease: "expo.inOut" }, 1.6);
tl.to(puzzle.root, { rotation: [null, "+=120", null], duration: 3, ease: "none" }, 0);

const letter = letterCube({ id: "l", size: 150, pos: [760, 270, 0], fill: "#8F6CFF", items: dot(150), letter: "A" });
letter.root.set({ rotation: [-12, -20, 0] });
tl.to(letter.root, { rotation: [null, "+=180", null], duration: 1.2, ease: "expo.inOut" }, 1.2);

film.timeline({ shots: [shot({ id: "cubes", dur: 3, layers: [light({ kind: "ambient" }), light({ kind: "directional", pos: [-300, -400, -600] }), a, puzzle, letter] })] });
export default film;
