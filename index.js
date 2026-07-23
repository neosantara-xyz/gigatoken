const { join } = require('path')

const nativeBinding = require(join(__dirname, 'gigatoken.node'))

module.exports = nativeBinding
