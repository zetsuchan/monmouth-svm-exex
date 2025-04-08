# Changelog

## [0.2.0] - 2024-03-20

### Added

- Enhanced SVM state management with `SvmState` struct
  - Added `AccountsDb` for managing Solana accounts
  - Added `ProgramCache` for caching compiled programs
- New `SvmProcessor` component with:
  - Transaction sanitization
  - Account loading
  - Instruction processing
  - eBPF VM execution
- Added `SvmBridge` for EVM-SVM interoperability:
  - Address mapping between EVM and Solana
  - Call translation
  - State bridging
- Implemented basic program caching mechanism
- Added account structure with Solana-compatible fields

### Changed

- Renamed `SvmExEx` to `EnhancedSvmExEx` for clarity
- Replaced simple bank simulation with full transaction processing pipeline
- Enhanced state management with proper account tracking
- Improved transaction processing with async support
- Updated main function to initialize new components

### Removed

- Removed `simulate_svm` function in favor of proper transaction processing
- Removed simplified `SharedSvmState` in favor of comprehensive `SvmState`

### Technical Debt

- TODO: Implement full transaction sanitization
- TODO: Add proper account locking mechanism
- TODO: Implement complete instruction processing
- TODO: Add state commitment logic
- TODO: Implement proper error handling for all components

## [0.1.0] - Initial Release

- Basic SVM execution extension
- Simple Solana bank integration
- Basic transaction simulation
