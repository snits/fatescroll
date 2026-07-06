// ABOUTME: Id generation for domain model entities (dirs, tables, results, chains).

export const uid = (): string => crypto.randomUUID();
