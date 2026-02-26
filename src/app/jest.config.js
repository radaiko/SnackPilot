module.exports = {
  testEnvironment: 'node',
  testMatch: ['**/src-rn/__tests__/**/*.test.ts'],
  setupFilesAfterEnv: ['./src-rn/__tests__/setup.ts'],
  transform: {
    '^.+\\.tsx?$': ['ts-jest', {
      tsconfig: 'tsconfig.json',
    }],
  },
  moduleNameMapper: {
    '.*/utils/analytics$': '<rootDir>/src-rn/utils/__mocks__/analytics.ts',
    '.*/utils/notificationLogStorage$': '<rootDir>/src-rn/utils/__mocks__/notificationLogStorage.ts',
  },
  moduleFileExtensions: ['ts', 'tsx', 'js', 'jsx', 'json'],
};
