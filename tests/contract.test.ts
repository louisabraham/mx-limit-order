import { test, beforeEach, afterEach } from "vitest";
import { assertAccount } from "xsuite/assert";
import { e } from "xsuite/data";
import { FWorld, FWorldWallet, FWorldContract } from "xsuite/world";

let world: FWorld;
let deployer: FWorldWallet;
let contract: FWorldContract;
let sender: FWorldWallet;
let receiver: FWorldWallet;
const egldId = "EGLD";
const sftId = "SFT-abcdef";

beforeEach(async () => {
    world = await FWorld.start();
    deployer = await world.createWallet();
    ({ contract } = await deployer.deployContract({
        code: "file:output/contract.wasm",
        codeMetadata: [],
        gasLimit: 10_000_000,
    }));
    sender = await world.createWallet({
        pairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 100_000 }])],
    });
    receiver = await world.createWallet({ balance: 100_000 });
});

afterEach(async () => {
    await world.terminate();
});

test("Test", async () => {
    assertAccount(await contract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    const contractId = (
        await sender.callContract({
            callee: contract,
            funcName: "create_order",
            funcArgs: [e.U(100_000)],
            esdts: [{ id: sftId, nonce: 1, amount: 100_000 }],
            gasLimit: 10_000_000,
        })
    ).returnData[0];
    console.log("contractId", contractId);

    assertAccount(await contract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 100_000 }])],
    });
    assertAccount(await sender.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    console.log("contract created successfully");
    await receiver.callContract({
        callee: contract,
        funcName: "fill_order",
        funcArgs: [contractId],
        value: 100_000,
        gasLimit: 10_000_000,
    });
    console.log("contract filled successfully");
    assertAccount(await sender.getAccountWithPairs(), {
        balance: 100_000n,
        hasPairs: [],
    });
    assertAccount(await contract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 0 }])],
    });
    assertAccount(await receiver.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 100_000 }])],
    });
});
