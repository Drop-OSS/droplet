import test from "ava";

import {
  generateRootCa,
  generateClientCertificate,
  verifyClientCertificate,
  signNonce,
  verifyNonce,
} from "../index.js";
import { randomUUID, sign } from "crypto";

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

  const [invalidPub, invalidPriv] = generateRootCa();

  const valid = verifyClientCertificate(clientPub, pub);
  if (valid) return t.pass();

  const invalid = verifyClientCertificate(invalidPub, pub);
  if (!invalid) return t.pass();

  return t.fail();
});

test("trust chain fails", (t) => {
  const [rootPub, rootPriv] = generateRootCa();

  const [clientPub, _priv] = generateClientCertificate(
    "",
    "",
    rootPub,
    rootPriv
  );

  const [otherRootPub, otherRootPriv] = generateRootCa();

  const valid = verifyClientCertificate(clientPub, otherRootPub);
  if (!valid) return t.pass();

  t.fail("client certificate verifies non-related certificate");
});

test("nonce signing", (t) => {
  const [pub, priv] = generateRootCa();
  const [clientPub, clientPriv] = generateClientCertificate(
    "test",
    "test",
    pub,
    priv
  );

  const nonce = randomUUID();
  const signature = signNonce(clientPriv, nonce);

  return t.pass();
});

test("nonce signing, and verification", (t) => {
  const [pub, priv] = generateRootCa();
  const [clientPub, clientPriv] = generateClientCertificate(
    "test",
    "test",
    pub,
    priv
  );

  const nonce = randomUUID();

  const signature = signNonce(clientPriv, nonce);
  const valid = verifyNonce(clientPub, nonce, signature);

  if (!valid) return t.fail("nonce does not verify correctly");

  return t.pass();
});

test("nonce signing, fails verification", (t) => {
  const [rootPub, rootPriv] = generateRootCa();
  const [clientPub, clientPriv] = generateClientCertificate(
    "test",
    "test",
    rootPub,
    rootPriv
  );
  const [otherClientPub, otherClientPriv] = generateClientCertificate("test2", "test2", rootPub, rootPriv);

  const nonce = randomUUID();
  const signature = signNonce(clientPriv, nonce);
  const valid = verifyNonce(otherClientPub, nonce, signature);

  if(valid) return t.fail("succesfully verified an invalid nonce");

  t.pass();
});
