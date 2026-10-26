import { comp } from "turbomotion";
import { __FUNCTION__ } from "./__NAME__/index";

const film = comp({ width: 1920, height: 1080, fps: 30 });
film.timeline({ shots: [__FUNCTION__({ id: "__NAME__", dur: 2 })] });
export default film;
