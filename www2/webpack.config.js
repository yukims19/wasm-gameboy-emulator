const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const outputDir = path.join(__dirname, 'build/');
// var CopyWebpackPlugin = require("copy-webpack-plugin");

const isProd = process.env.NODE_ENV === 'production';

module.exports = {
  entry: './src/bootstrap.js',
  mode: isProd ? 'production' : 'development',
  output: {
    path: outputDir,
    filename: 'Index.js'
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: 'src/index.html',
      inject: false
    }),
    // new CopyPlugin([
    //   { from: 'src/bootstrap.js', to: outputDir  },
    // ]),
  ],
  devServer: {
    compress: true,
    contentBase: outputDir,
    port: process.env.PORT || 8002,
    historyApiFallback: true
  }
};
