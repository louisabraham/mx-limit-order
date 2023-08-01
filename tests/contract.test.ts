import { test, beforeEach, afterEach } from "vitest";
import { assertAccount } from "xsuite/assert";
import { e } from "xsuite/data";
import { FWorld, FWorldWallet, FWorldContract } from "xsuite/world";

let world: FWorld;
let deployer: FWorldWallet;
let contract: FWorldContract;
let otherContract: FWorldContract;
let sender: FWorldWallet;
let receiver: FWorldWallet;
let arbitrageur: FWorldWallet;
const egldId = "EGLD";
const sftId = "SFT-abcdef";

beforeEach(async () => {
    world = await FWorld.start();
    deployer = await world.createWallet();
    ({ contract } = await deployer.deployContract({
        code: "file:output/limit-order.wasm",
        codeMetadata: [],
        gasLimit: 10_000_000,
    }));
    otherContract = (
        await deployer.deployContract({
            code: "file:output/limit-order.wasm",
            codeMetadata: [],
            gasLimit: 10_000_000,
        })
    ).contract;
    sender = await world.createWallet({
        pairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 100_000 }])],
    });
    receiver = await world.createWallet({ balance: 100_000 });
    arbitrageur = await world.createWallet();
});

afterEach(async () => {
    await world.terminate();
});

test("Test fill order", async () => {
    assertAccount(await contract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    const contractId = (
        await sender.callContract({
            callee: contract,
            funcName: "create_order",
            funcArgs: [e.Tuple(e.Str(egldId), e.U64(0), e.U(100_000))],
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
    console.log(
        await sender.getAccountWithPairs(),
        await receiver.getAccountWithPairs()
    );
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

test("Test fill order with other", async () => {
    assertAccount(await contract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    [sender, receiver] = [receiver, sender];

    const contractId1 = (
        await sender.callContract({
            callee: contract,
            funcName: "create_order",
            funcArgs: [e.Tuple(e.Str(sftId), e.U64(1), e.U(100_000))],
            value: 100_000,
            gasLimit: 10_000_000,
        })
    ).returnData[0];
    console.log("contract 1 created successfully");

    const contractId2 = (
        await receiver.callContract({
            callee: otherContract,
            funcName: "create_order",
            funcArgs: [e.Tuple(e.Str(egldId), e.U64(0), e.U(90_000))],
            esdts: [{ id: sftId, nonce: 1, amount: 100_000 }],
            gasLimit: 10_000_000,
        })
    ).returnData[0];
    console.log("contract 2 created successfully");

    assertAccount(await contract.getAccountWithPairs(), {
        balance: 100_000n,
        hasPairs: [],
    });
    assertAccount(await otherContract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 100_000 }])],
    });
    assertAccount(await sender.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    assertAccount(await receiver.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    assertAccount(await arbitrageur.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });

    await arbitrageur.callContract({
        callee: contract,
        funcName: "fill_order_with_other",
        funcArgs: [
            contractId1,
            otherContract,
            contractId2,
            e.Tuple(e.Str(egldId), e.U64(0), e.U(90_000)),
        ],
        gasLimit: 10_000_000,
    });
    console.log("contract filled successfully");

    console.log(
        await sender.getAccountWithPairs(),
        await receiver.getAccountWithPairs(),
        await arbitrageur.getAccountWithPairs(),
        (await contract.getAccountWithPairs()).balance,
        (await otherContract.getAccountWithPairs()).balance
    );
    assertAccount(await contract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 0 }])],
    });
    assertAccount(await otherContract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 0 }])],
    });

    assertAccount(await sender.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 100_000 }])],
    });
    assertAccount(await receiver.getAccountWithPairs(), {
        balance: 90_000n,
        hasPairs: [],
    });
    assertAccount(await arbitrageur.getAccountWithPairs(), {
        balance: 10_000n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 0 }])],
    });
});

test("Test fill order with self", async () => {
    assertAccount(await contract.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    [sender, receiver] = [receiver, sender];
    otherContract = contract;
    const contractId1 = (
        await sender.callContract({
            callee: contract,
            funcName: "create_order",
            funcArgs: [e.Tuple(e.Str(sftId), e.U64(1), e.U(100_000))],
            value: 100_000,
            gasLimit: 10_000_000,
        })
    ).returnData[0];
    console.log("contract 1 created successfully");

    const contractId2 = (
        await receiver.callContract({
            callee: otherContract,
            funcName: "create_order",
            funcArgs: [e.Tuple(e.Str(egldId), e.U64(0), e.U(90_000))],
            esdts: [{ id: sftId, nonce: 1, amount: 100_000 }],
            gasLimit: 10_000_000,
        })
    ).returnData[0];
    console.log("contract 2 created successfully");

    assertAccount(await sender.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    assertAccount(await receiver.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });
    assertAccount(await arbitrageur.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [],
    });

    await arbitrageur.callContract({
        callee: contract,
        funcName: "fill_order_with_other",
        funcArgs: [
            contractId1,
            otherContract,
            contractId2,
            e.Tuple(e.Str(egldId), e.U64(0), e.U(90_000)),
        ],
        gasLimit: 10_000_000,
    });
    console.log("contract filled successfully");

    console.log(
        await sender.getAccountWithPairs(),
        await receiver.getAccountWithPairs(),
        await arbitrageur.getAccountWithPairs(),
        (await contract.getAccountWithPairs()).balance,
        (await otherContract.getAccountWithPairs()).balance
    );

    assertAccount(await sender.getAccountWithPairs(), {
        balance: 0n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 100_000 }])],
    });
    assertAccount(await receiver.getAccountWithPairs(), {
        balance: 90_000n,
        hasPairs: [],
    });
    assertAccount(await arbitrageur.getAccountWithPairs(), {
        balance: 10_000n,
        hasPairs: [e.p.Esdts([{ id: sftId, nonce: 1, amount: 0 }])],
    });
});
