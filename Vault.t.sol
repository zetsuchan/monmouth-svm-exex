// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import {Test, console} from "forge-std/Test.sol";
import {UnsafeVault, ReentrancyAttacker} from "../Vault.sol";

/**
 * @title UnsafeVaultTest
 * @author Gemini
 * @notice This test suite is designed to audit the UnsafeVault contract.
 * It explores edge cases and vulnerabilities that can lead to loss of funds or
 * balance inconsistencies. We will use this to iteratively find and fix bugs.
 */
contract UnsafeVaultTest is Test {
    UnsafeVault public vault;
    address public user1 = address(1);
    address public user2 = address(2);

    // Initial fund amounts for the tests
    uint256 constant USER1_DEPOSIT = 10 ether;
    uint256 constant USER2_DEPOSIT = 5 ether;

    function setUp() public {
        // Deploy the vault contract to be tested.
        vault = new UnsafeVault();

        // Fund the users with Ether for the tests.
        vm.deal(user1, 20 ether);
        vm.deal(user2, 10 ether);
    }

    /**
     * @notice Test Case 1: Basic Deposit and Withdraw
     * This test checks the fundamental functionality: can a user deposit and then
     * withdraw their own funds successfully?
     */
    function test_Audit_NormalDepositAndWithdraw() public {
        // User 1 deposits 10 Ether.
        vm.startPrank(user1);
        vault.deposit{value: USER1_DEPOSIT}();
        assertEq(vault.balances(user1), USER1_DEPOSIT, "User 1 balance should be 10 ether");
        assertEq(vault.totalVaultBalance(), USER1_DEPOSIT, "Total vault balance should be 10 ether");
        vm.stopPrank();

        // User 1 withdraws 10 Ether.
        vm.startPrank(user1);
        vault.withdraw(USER1_DEPOSIT);
        vm.stopPrank();

        // Final state check: Balances should be zero, and the vault should be empty.
        assertEq(vault.balances(user1), 0, "User 1 balance should be zero after withdrawal");
        assertEq(vault.totalVaultBalance(), 0, "Total vault balance should be zero");
        assertEq(vault.getBalance(), 0, "Contract ETH balance should be zero");
    }

    /**
     * @notice Test Case 2: Reentrancy Attack
     * This test demonstrates how a malicious contract can drain the vault's funds
     * by exploiting the reentrancy vulnerability in the `withdraw` function.
     */
    function test_Audit_ReentrancyExploit() public {
        // Deploy the attacker contract.
        ReentrancyAttacker attacker = new ReentrancyAttacker(address(vault));

        // Innocent users deposit funds into the vault.
        vm.startPrank(user1);
        vault.deposit{value: USER1_DEPOSIT}();
        vm.stopPrank();

        vm.startPrank(user2);
        vault.deposit{value: USER2_DEPOSIT}();
        vm.stopPrank();

        // The vault now has 15 Ether.
        assertEq(vault.getBalance(), USER1_DEPOSIT + USER2_DEPOSIT, "Vault should have 15 ether");

        // The attacker funds their contract and deposits 1 Ether into the vault.
        vm.deal(address(attacker), 1 ether);
        attacker.depositToVault(1 ether);

        // The vault now has 16 Ether.
        assertEq(vault.getBalance(), 16 ether, "Vault should have 16 ether before attack");

        // The attacker initiates the reentrancy attack.
        attacker.attack();

        // Post-attack assertions.
        console.log("Vault balance after attack:", vault.getBalance());
        console.log("Attacker balance after attack:", attacker.getBalance());

        // The vault should be completely drained.
        assertTrue(vault.getBalance() == 0, "FAIL: Vault was not drained!");
        // The attacker should have all the funds.
        assertTrue(attacker.getBalance() > 15 ether, "FAIL: Attacker did not drain funds!");
    }

    /**
     * @notice Test Case 3: Integer Underflow
     * This test shows how a user can withdraw more than they deposited due to an
     * integer underflow vulnerability. The user withdraws their balance twice.
     */
    function test_Audit_IntegerUnderflowExploit() public {
        // User 1 deposits 10 Ether.
        vm.startPrank(user1);
        vault.deposit{value: USER1_DEPOSIT}();
        vm.stopPrank();

        // The user withdraws their balance twice.
        // The second withdrawal should fail, but due to the underflow, it succeeds.
        vm.startPrank(user1);
        vault.withdraw(USER1_DEPOSIT);
        vault.withdraw(USER1_DEPOSIT); // This should fail
        vm.stopPrank();

        // The user's balance should be a very large number due to the underflow.
        assertTrue(vault.balances(user1) > 10**20, "FAIL: Integer underflow did not occur!");

        // The total vault balance is now incorrect.
        console.log("Total vault balance after underflow:", vault.totalVaultBalance());
        assertTrue(vault.totalVaultBalance() != 0, "FAIL: Total vault balance is incorrect!");
    }

    /**
     * @notice Test Case 4: Unchecked Call Return Value
     * This test demonstrates the risk of not checking the return value of a `.call`.
     * We simulate a failed call and observe that the user's balance is still debited.
     */
    function test_Audit_UncheckedCall() public {
        // A contract that cannot receive Ether.
        contract NoReceive {}
        address payable noReceiveContract = payable(address(new NoReceive()));

        // User 1 deposits 10 Ether.
        vm.startPrank(user1);
        vault.deposit{value: USER1_DEPOSIT}();
        vm.stopPrank();

        // We will try to withdraw to the contract that cannot receive Ether.
        // To do this, we need to temporarily change the user's balance mapping.
        // This is a bit of a hack, but it allows us to test the vulnerability.
        // We will use vm.store to directly write to the storage slot of the mapping.
        bytes32 slot = keccak256(abi.encodePacked(user1, uint256(0))); // slot for balances[user1]
        vm.store(address(vault), slot, bytes32(uint256(USER1_DEPOSIT)));

        // Now, we try to withdraw to the noReceiveContract.
        // The call will fail, but the balance will still be debited.
        vm.startPrank(user1);
        vault.withdraw(USER1_DEPOSIT);
        vm.stopPrank();

        // The user's balance should be zero, even though the transfer failed.
        assertEq(vault.balances(user1), 0, "User 1 balance should be zero");

        // The vault's total balance is now incorrect.
        assertEq(vault.totalVaultBalance(), 0, "Total vault balance is incorrect");

        // The Ether is still in the vault.
        assertEq(vault.getBalance(), USER1_DEPOSIT, "Vault should still have the Ether");
    }
}
