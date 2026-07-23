export class GigaTokenizer {
  static fromTiktoken(path: string): GigaTokenizer
  static fromTiktokenModel(modelPath: string, configPath: string, pretokenizer: string): GigaTokenizer
  static fromHf(path: string): GigaTokenizer
  encode(text: string): number[]
  countTokens(text: string): number
  countTokensBatch(inputs: string[]): number[]
}
