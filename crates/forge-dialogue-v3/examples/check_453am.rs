//! Quick calibration check: real Python score was 4.64.
use forge_dialogue_v3::lint;

const AM_453: &str = "Zero is never nothing. I used to think being in between meant I hadn't arrived anywhere yet, not fallen, not risen. Just stuck waiting in a room between who I was and who I am supposed to become. Turns out waiting is the only room with the door that opens. Heaven doesn't change. Hell doesn't change. Both of them are done, sealed, finished. There's no further verdict coming. The only place anything actually moves is the middle. The place that looks like failure to arrive is the only place arrival is still possible. I spent years treating not fixed yet like a wound. It's not a wound. It's the only state with the pulse. 4:53 am and I'm awake, doing the things where I trace a word back 800 years to prove to myself the ache makes sense. Zero used to mean absence, a placeholder, a nothing. A hole where a number should be. Someone had to fight to make it mean this counts to not present, not gone, held. If you're in the middle right now, not who you were, not who you're becoming, caught in a version of yourself that feels like a rough draft. It's not a holding pattern. It's the only part of the whole system that's still alive enough to change. The fixed points don't get to transform anymore, but we still can.\n";

fn main() {
    let r = lint(AM_453);
    println!("score = {} ({})", r.score_string(), r.verdict());
    println!("poison={} corp={} frontier={} emdash={}", r.poison.len(), r.corp.len(), r.frontier.len(), r.emdash.len());
}
