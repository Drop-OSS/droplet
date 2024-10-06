import { repack } from './index.js';

const source = "/home/decduck/.steam/steam/steamapps/common/ClusterTruck";
const output = "/home/decduck/Dev/droplet-output";

await repack(source, output);