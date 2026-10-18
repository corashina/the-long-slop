import {readFile} from 'node:fs/promises';

const runs = JSON.parse(await readFile(process.argv[2], 'utf8'));
const groups = new Map();
const identity = JSON.stringify([runs[0].canvas, runs[0].dpr, runs[0].source]);
for (const run of runs) {
  if (JSON.stringify([run.canvas, run.dpr, run.source]) !== identity) throw new Error('Benchmark viewport or source identity differs.');
  const key = `${run.variant} / ${run.scene} / ${run.preparation ? 'profile' : 'FPS'}`;
  if (!groups.has(key)) groups.set(key, []);
  groups.get(key).push(run);
}
const median = values => {
  values.sort((a, b) => a - b);
  const middle = values.length >> 1;
  return values.length % 2 ? values[middle] : (values[middle - 1] + values[middle]) / 2;
};
console.log('| Run | Samples | FPS median | Frame mean ms | Frame p95 ms | Preparation ms/frame |');
console.log('|---|---:|---:|---:|---:|---:|');
for (const [key, samples] of groups) {
  const prep = samples.filter(run => run.preparation).map(run => {
    const rows = run.measurements.measurements;
    const frames = rows.reduce((sum, row) => sum + row[0].calls, 0);
    const ms = rows.reduce((sum, row) => sum + row.reduce((total, stage) => total + stage.total_ms, 0), 0);
    return ms / frames;
  });
  console.log(`| ${key} | ${samples.length} | ${median(samples.map(run => run.fps)).toFixed(2)} | ${median(samples.map(run => run.frameMs.mean)).toFixed(2)} | ${median(samples.map(run => run.frameMs.p95)).toFixed(2)} | ${prep.length ? median(prep).toFixed(3) : '-'} |`);
}
for (const run of runs.filter(run => run.preparation)) {
  console.log(`\n${run.variant} / ${run.scene}: prepared faces by job and stage`);
  const profile = run.measurements;
  for (let job = 0; job < profile.jobs.length; job++) {
    const row = profile.measurements[job];
    if (!row.some(stage => stage.calls)) continue;
    console.log(profile.jobs[job], row.map((stage, index) => `${profile.stages[index]}=${stage.faces}/${stage.calls} [${stage.face_counts.join(',')}]`).join(' '));
  }
}
