import test from "ava";

import {
  generateRootCa,
  generateClientCertificate,
  verifyClientCertificate,
} from "../index.js";

test("generate ca", (t) => {
  const [pub, priv] = generateRootCa();
  t.pass();
});

test("generate ca & client certs", (t) => {
  const [pub, priv] = generateRootCa();

  const clientName = "My Test Client";
  const [clientPub, clientPriv] = generateClientCertificate(
    clientName,
    clientName,
    pub,
    priv
  );

  t.pass();
});

test("trust chain", (t) => {
  const [pub, priv] = generateRootCa();

  const clientName = "My Test Client";
  const [clientPub, clientPriv] = generateClientCertificate(
    clientName,
    clientName,
    pub,
    priv
  );

  const valid = verifyClientCertificate(clientPub, pub);
  if (valid) return t.pass();
  return t.fail();
});
