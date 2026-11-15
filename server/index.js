'use strict';
const { createApp } = require('./app');
const port = Number(process.env.PORT || 8787);
const { server } = createApp({ DB_PATH: process.env.DB_PATH || '/data/c02.sqlite' });
server.listen(port, '0.0.0.0', () => console.log('C02 Naval API sur le port ' + port));
for (const sig of ['SIGTERM', 'SIGINT']) process.on(sig, () => server.close(() => process.exit(0)));
