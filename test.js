const droplet = require('.');
const fs = require('fs');

(async () => {
    const manifest = await droplet.generateManifest("/home/decduck/Games/STAR WARS Jedi Survivor", console.log, console.log);
    fs.writeFileSync('./manifest.json', manifest);
})();
